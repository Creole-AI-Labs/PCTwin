//! Waiting sends (Security Design Part T, Quick Send Stage 1): "approve this delivery only".
//!
//! - One approval binds the exact file versions (each file's BLAKE3 fingerprint), each
//!   recipient's own key (a remembered connection's public key), the destination (that recipient
//!   only: where it lands is the recipient's to choose) and a deadline ([`DEFAULT_DAYS`] days,
//!   renewed only when the person asks, [`WaitingStore::renew`]).
//! - The approved versions are copied into PCTwin's own folder on this laptop: **Preparing**
//!   until every copy is made and fingerprinted, then **Waiting**. What is sent is that copy, its
//!   fingerprints checked again just before each delivery; a copy that no longer matches is never
//!   sent. A source changed since only says "a newer version exists, review again": it is never
//!   swapped in.
//! - Several recipients: one approval lists each recipient's key; there is no way to add one to
//!   an approval (a new approval is needed); each recipient gets its own session and its own
//!   result, and its own offer ID, so two recipients cannot tell from what they received that
//!   they got the same send.
//! - The copies have a total cap ([`WaitingStore::with_cap`]), need free space first (keeping
//!   [`KEEP_FREE`] spare), show as used space ([`WaitingStore::used_bytes`]), and are deleted at
//!   the deadline ([`WaitingStore::sweep`]) or on cancel. On an SSD deleting is not a guaranteed
//!   wipe (Part G).
//! - The deadline is this laptop's clock; a waiting send delivers only while this laptop is on.
//!
//! Protected storage (Part G): the copies are the person's own files, kept in PCTwin's folder
//! under the person's account (on Mac and Linux made `0700`), not encrypted; nothing secret is
//! kept beside them.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use pctwin_record::{ItemKind, LaptopId};
use pctwin_transfer::{Channel, KEEP_FREE};
use serde::{Deserialize, Serialize};

use crate::{Offer, Outcome, QuickError, Receipt};

const DAY: u64 = 24 * 60 * 60;
/// The deadline unless the person chose another.
pub const DEFAULT_DAYS: u64 = 7;
/// The longest deadline one approval or renewal may set.
pub const MOST_DAYS: u64 = 30;
/// The copies' total cap unless set otherwise.
pub const DEFAULT_CAP: u64 = 20 * 1024 * 1024 * 1024;
/// Most recipients in one approval.
pub const MOST_RECIPIENTS: usize = 20;
const MANIFEST: &str = "waiting.json";
const FILES: &str = "files";
const VERSION: u32 = 1;

/// Why a waiting send could not be made, or delivered.
#[derive(Debug, thiserror::Error)]
pub enum WaitingError {
    #[error("no recipient, too many, or one named twice")]
    Recipients,
    #[error("the deadline must be 1 to {MOST_DAYS} days")]
    Days,
    #[error("the waiting sends would use more than their limit: {needed} bytes more, {room} left")]
    OverCap { needed: u64, room: u64 },
    #[error("not enough room: {needed} bytes needed, {room} bytes can be used")]
    NoRoom { needed: u64, room: u64 },
    #[error("a file changed while it was being copied: {0}")]
    ChangedWhileCopying(String),
    /// The protected copy no longer matches what was approved: it is not sent.
    #[error("the copy kept for this send no longer matches what was approved")]
    CopyDamaged,
    #[error("no such waiting send")]
    NotFound,
    #[error("this waiting send has passed its deadline")]
    PastDeadline,
    #[error("the waiting send could not be read or saved")]
    File,
    #[error(transparent)]
    Quick(#[from] QuickError),
}

/// Where a waiting send is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum State {
    /// The approved versions are being copied.
    Preparing,
    /// Copied and fingerprinted, waiting for its recipients.
    Waiting,
}

/// One approved file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaitingFile {
    /// Its path in the send, `/` between folders.
    pub path: String,
    /// Where the copy is, inside the send's folder, part by part.
    parts: Vec<PathBuf>,
    pub size: u64,
    /// The approved version's BLAKE3 fingerprint, hex.
    pub fingerprint: String,
    /// Where it was read from, to tell when a newer version exists.
    source: PathBuf,
    source_modified: Option<u128>,
}

/// How a delivery to one recipient came out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum Delivery {
    /// Not delivered yet (or not answered yet: tried again later).
    Waiting,
    /// Delivered: how many files arrived (or were already there), and how many need the
    /// person's care there, at `at`.
    Delivered {
        arrived: u64,
        needs_action: u64,
        at: u64,
    },
    /// The recipient declined it.
    Declined { at: u64 },
    /// Some files did not arrive: how many, and the words.
    Failed { failed: u64, why: String, at: u64 },
}

/// One recipient: a remembered connection, by its id and the key it had when this was approved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recipient {
    pub connection: String,
    /// The recipient's public key, hex: delivery goes only to this key.
    pub key: String,
    /// The offer ID this recipient's offers are made under (its own, see the module notes).
    laptop: String,
    pub delivery: Delivery,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Manifest {
    version: u32,
    id: String,
    made_at: u64,
    deadline: u64,
    state: State,
    /// The chosen things' top-level names, in the order chosen.
    tops: Vec<PathBuf>,
    files: Vec<WaitingFile>,
    recipients: Vec<Recipient>,
}

/// A waiting send as the screens list it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaitingInfo {
    pub id: String,
    pub made_at: u64,
    pub deadline: u64,
    pub state: State,
    pub files: u64,
    pub bytes: u64,
    pub recipients: Vec<Recipient>,
    /// Paths whose source changed since the approval: "a newer version exists, review again".
    pub newer: Vec<String>,
}

/// PCTwin's folder of waiting sends.
pub struct WaitingStore {
    root: PathBuf,
    cap: u64,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn modified(meta: &std::fs::Metadata) -> Option<u128> {
    meta.modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_nanos())
}

/// Copies `from` to the new file `to`, fingerprinting what is written; the bytes copied and the
/// fingerprint.
fn copy_fingerprinted(from: &Path, to: &Path) -> std::io::Result<(u64, String)> {
    let mut src = std::fs::File::open(from)?;
    let mut dst = std::fs::File::options()
        .write(true)
        .create_new(true)
        .open(to)?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0u8; 256 * 1024];
    let mut n = 0u64;
    loop {
        let got = src.read(&mut buf)?;
        if got == 0 {
            break;
        }
        hasher.update(&buf[..got]);
        dst.write_all(&buf[..got])?;
        n += got as u64;
    }
    dst.sync_all()?;
    Ok((n, hex(hasher.finalize().as_bytes())))
}

fn fingerprint_of(path: &Path) -> std::io::Result<(u64, String)> {
    let mut f = std::fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0u8; 256 * 1024];
    let mut n = 0u64;
    loop {
        let got = f.read(&mut buf)?;
        if got == 0 {
            break;
        }
        hasher.update(&buf[..got]);
        n += got as u64;
    }
    Ok((n, hex(hasher.finalize().as_bytes())))
}

fn make_private_dir(dir: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)
    }
    #[cfg(not(unix))]
    {
        std::fs::create_dir_all(dir)
    }
}

impl WaitingStore {
    /// The waiting sends kept in `root` (PCTwin's own folder for them), with [`DEFAULT_CAP`].
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            cap: DEFAULT_CAP,
        }
    }

    /// With a total cap of `cap` bytes for every waiting send's copies together.
    pub fn with_cap(mut self, cap: u64) -> Self {
        self.cap = cap;
        self
    }

    fn dir(&self, id: &str) -> Result<PathBuf, WaitingError> {
        if id.len() != 32 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(WaitingError::NotFound);
        }
        Ok(self.root.join(id))
    }

    fn read(&self, id: &str) -> Result<Manifest, WaitingError> {
        let bytes =
            std::fs::read(self.dir(id)?.join(MANIFEST)).map_err(|_| WaitingError::NotFound)?;
        let m: Manifest = serde_json::from_slice(&bytes).map_err(|_| WaitingError::File)?;
        if m.version != VERSION || m.id != id {
            return Err(WaitingError::File);
        }
        Ok(m)
    }

    fn write(&self, m: &Manifest) -> Result<(), WaitingError> {
        let dir = self.dir(&m.id)?;
        let bytes = serde_json::to_vec_pretty(m).map_err(|_| WaitingError::File)?;
        let part = dir.join(format!("{MANIFEST}.part"));
        let mut f = std::fs::File::create(&part).map_err(|_| WaitingError::File)?;
        f.write_all(&bytes)
            .and_then(|()| f.sync_all())
            .map_err(|_| WaitingError::File)?;
        drop(f);
        std::fs::rename(&part, dir.join(MANIFEST)).map_err(|_| WaitingError::File)
    }

    fn ids(&self) -> Vec<String> {
        let Ok(entries) = std::fs::read_dir(&self.root) else {
            return Vec::new();
        };
        let mut ids: Vec<String> = entries
            .filter_map(Result::ok)
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| n.len() == 32 && n.bytes().all(|b| b.is_ascii_hexdigit()))
            .collect();
        ids.sort();
        ids
    }

    /// The bytes every waiting send's copies use, as their approved sizes say.
    pub fn used_bytes(&self) -> u64 {
        self.ids()
            .iter()
            .filter_map(|id| self.read(id).ok())
            .flat_map(|m| m.files.into_iter().map(|f| f.size))
            .fold(0u64, u64::saturating_add)
    }

    /// Approve this delivery only: the `chosen` files and folders, as they are now, for each of
    /// `recipients` (a connection id and its public key), until `days` days after `now`. Checks
    /// the cap and `free` space (`None`: the drive cannot say) before copying anything; copies
    /// and fingerprints each file (Preparing), then marks it Waiting. A file that changed while
    /// it was copied ends it, nothing kept.
    pub fn prepare(
        &self,
        chosen: &[PathBuf],
        recipients: &[(String, [u8; 32])],
        days: u64,
        now: u64,
        free: Option<u64>,
    ) -> Result<WaitingInfo, WaitingError> {
        if recipients.is_empty() || recipients.len() > MOST_RECIPIENTS {
            return Err(WaitingError::Recipients);
        }
        let connections: std::collections::BTreeSet<&String> =
            recipients.iter().map(|(c, _)| c).collect();
        let keys: std::collections::BTreeSet<&[u8; 32]> =
            recipients.iter().map(|(_, k)| k).collect();
        if connections.len() != recipients.len() || keys.len() != recipients.len() {
            return Err(WaitingError::Recipients);
        }
        if days == 0 || days > MOST_DAYS {
            return Err(WaitingError::Days);
        }
        let offer = Offer::of(chosen)?;
        let (record, jobs) = offer.into_parts();
        let items: BTreeMap<_, _> = record.items.iter().map(|i| (i.id, i)).collect();
        let total = jobs.iter().fold(0u64, |t, j| {
            t.saturating_add(items.get(&j.item).map_or(0, |i| i.size_bytes))
        });
        let used = self.used_bytes();
        if used.saturating_add(total) > self.cap {
            return Err(WaitingError::OverCap {
                needed: total,
                room: self.cap.saturating_sub(used),
            });
        }
        if let Some(free) = free
            && total.saturating_add(KEEP_FREE) > free
        {
            return Err(WaitingError::NoRoom {
                needed: total,
                room: free.saturating_sub(KEEP_FREE),
            });
        }
        let id = LaptopId::generate().map_err(QuickError::from)?.to_hex();
        let dir = self.dir(&id)?;
        make_private_dir(&dir.join(FILES)).map_err(|_| WaitingError::File)?;
        let mut recipients_out = Vec::new();
        for (connection, key) in recipients {
            recipients_out.push(Recipient {
                connection: connection.clone(),
                key: hex(key),
                laptop: LaptopId::generate().map_err(QuickError::from)?.to_hex(),
                delivery: Delivery::Waiting,
            });
        }
        let mut m = Manifest {
            version: VERSION,
            id: id.clone(),
            made_at: now,
            deadline: now.saturating_add(days * DAY),
            state: State::Preparing,
            tops: Vec::new(),
            files: Vec::new(),
            recipients: recipients_out,
        };
        self.write(&m)?;
        let copied = (|| {
            for job in &jobs {
                let Some(item) = items.get(&job.item) else {
                    continue;
                };
                if item.kind != ItemKind::File {
                    continue;
                }
                let parts: Vec<PathBuf> = item
                    .path
                    .parts()
                    .iter()
                    .map(|p| PathBuf::from(p.on_disk()))
                    .collect();
                let mut to = dir.join(FILES);
                for p in &parts {
                    to.push(p);
                }
                if let Some(parent) = to.parent() {
                    make_private_dir(parent).map_err(|_| WaitingError::File)?;
                }
                let before = std::fs::metadata(&job.source)
                    .map_err(|_| WaitingError::ChangedWhileCopying(job.path.clone()))?;
                let (n, fingerprint) = copy_fingerprinted(&job.source, &to)
                    .map_err(|_| WaitingError::ChangedWhileCopying(job.path.clone()))?;
                let after = std::fs::metadata(&job.source)
                    .map_err(|_| WaitingError::ChangedWhileCopying(job.path.clone()))?;
                if n != before.len() || after.len() != n || modified(&before) != modified(&after) {
                    return Err(WaitingError::ChangedWhileCopying(job.path.clone()));
                }
                m.files.push(WaitingFile {
                    path: job.path.clone(),
                    parts,
                    size: n,
                    fingerprint,
                    source: job.source.clone(),
                    source_modified: modified(&after),
                });
            }
            Ok(())
        })();
        if let Err(e) = copied {
            let _ = std::fs::remove_dir_all(&dir);
            return Err(e);
        }
        // The copy's own top-level names (two chosen things with one name were kept apart).
        for f in &m.files {
            if let Some(top) = f.parts.first()
                && !m.tops.contains(top)
            {
                m.tops.push(top.clone());
            }
        }
        m.state = State::Waiting;
        if let Err(e) = self.write(&m) {
            let _ = std::fs::remove_dir_all(&dir);
            return Err(e);
        }
        Ok(self.info(&m))
    }

    fn info(&self, m: &Manifest) -> WaitingInfo {
        let newer = m
            .files
            .iter()
            .filter(|f| match std::fs::metadata(&f.source) {
                Ok(meta) => meta.len() != f.size || modified(&meta) != f.source_modified,
                Err(_) => true,
            })
            .map(|f| f.path.clone())
            .collect();
        WaitingInfo {
            id: m.id.clone(),
            made_at: m.made_at,
            deadline: m.deadline,
            state: m.state,
            files: m.files.len() as u64,
            bytes: m.files.iter().fold(0u64, |t, f| t.saturating_add(f.size)),
            recipients: m.recipients.clone(),
            newer,
        }
    }

    /// Every waiting send, oldest first. One left Preparing (PCTwin stopped while copying) is
    /// removed: it was never approved as complete.
    pub fn list(&self) -> Vec<WaitingInfo> {
        let mut out = Vec::new();
        for id in self.ids() {
            match self.read(&id) {
                Ok(m) if m.state == State::Waiting => out.push(self.info(&m)),
                _ => {
                    if let Ok(dir) = self.dir(&id) {
                        let _ = std::fs::remove_dir_all(dir);
                    }
                }
            }
        }
        out.sort_by_key(|w| (w.made_at, w.id.clone()));
        out
    }

    /// One waiting send.
    pub fn get(&self, id: &str) -> Result<WaitingInfo, WaitingError> {
        let m = self.read(id)?;
        if m.state != State::Waiting {
            return Err(WaitingError::NotFound);
        }
        Ok(self.info(&m))
    }

    /// Renews the deadline to `days` days after `now`, only when the person asks.
    pub fn renew(&self, id: &str, days: u64, now: u64) -> Result<(), WaitingError> {
        if days == 0 || days > MOST_DAYS {
            return Err(WaitingError::Days);
        }
        let mut m = self.read(id)?;
        m.deadline = now.saturating_add(days * DAY);
        self.write(&m)
    }

    /// Cancels a waiting send: its copies are deleted.
    pub fn cancel(&self, id: &str) -> Result<(), WaitingError> {
        let dir = self.dir(id)?;
        if !dir.exists() {
            return Err(WaitingError::NotFound);
        }
        std::fs::remove_dir_all(&dir).map_err(|_| WaitingError::File)
    }

    /// Ends every delivery to connection `connection` (it was forgotten): it is taken off each
    /// waiting send; a send left with no recipient is deleted. Returns how many sends changed.
    pub fn forget_recipient(&self, connection: &str) -> usize {
        let mut changed = 0;
        for id in self.ids() {
            let Ok(mut m) = self.read(&id) else { continue };
            let before = m.recipients.len();
            m.recipients.retain(|r| r.connection != connection);
            if m.recipients.len() == before {
                continue;
            }
            changed += 1;
            if m.recipients.is_empty() {
                let _ = self.cancel(&id);
            } else {
                let _ = self.write(&m);
            }
        }
        changed
    }

    /// Deletes every waiting send past its deadline at `now`, and any left Preparing; returns
    /// the ids deleted.
    pub fn sweep(&self, now: u64) -> Vec<String> {
        let mut gone = Vec::new();
        for id in self.ids() {
            let expired = match self.read(&id) {
                Ok(m) => m.state != State::Waiting || now >= m.deadline,
                Err(_) => true,
            };
            if expired && self.cancel(&id).is_ok() {
                gone.push(id);
            }
        }
        gone
    }

    /// The copy checked against the approval: every file there, with its approved size and
    /// fingerprint, and nothing else in the folder.
    fn verify(&self, m: &Manifest) -> Result<(), WaitingError> {
        let files = self.dir(&m.id)?.join(FILES);
        let mut expected = std::collections::BTreeSet::new();
        for f in &m.files {
            let mut at = files.clone();
            for p in &f.parts {
                at.push(p);
            }
            let (n, fp) = fingerprint_of(&at).map_err(|_| WaitingError::CopyDamaged)?;
            if n != f.size || fp != f.fingerprint {
                return Err(WaitingError::CopyDamaged);
            }
            expected.insert(at);
        }
        // Nothing added beside the approved files.
        let mut stack = vec![files];
        while let Some(dir) = stack.pop() {
            for e in std::fs::read_dir(&dir).map_err(|_| WaitingError::CopyDamaged)? {
                let e = e.map_err(|_| WaitingError::CopyDamaged)?;
                let t = e.file_type().map_err(|_| WaitingError::CopyDamaged)?;
                if t.is_dir() {
                    stack.push(e.path());
                } else if !t.is_file() || !expected.contains(&e.path()) {
                    return Err(WaitingError::CopyDamaged);
                }
            }
        }
        Ok(())
    }

    /// The offer of the protected copy for recipient `connection`, checked against the approval
    /// just now: the same files, sizes and fingerprints. Also the key it may go to.
    pub fn offer_for(
        &self,
        id: &str,
        connection: &str,
        now: u64,
    ) -> Result<(Offer, [u8; 32]), WaitingError> {
        let m = self.read(id)?;
        if m.state != State::Waiting {
            return Err(WaitingError::NotFound);
        }
        if now >= m.deadline {
            return Err(WaitingError::PastDeadline);
        }
        let r = m
            .recipients
            .iter()
            .find(|r| r.connection == connection)
            .ok_or(WaitingError::NotFound)?;
        self.verify(&m)?;
        let files = self.dir(id)?.join(FILES);
        let tops: Vec<PathBuf> = m.tops.iter().map(|t| files.join(t)).collect();
        let laptop = LaptopId::from_hex(&r.laptop).map_err(|_| WaitingError::File)?;
        let offer = Offer::of_again(&tops, laptop)?;
        // The offer must list exactly the approved files.
        let offered: BTreeMap<String, u64> = offer
            .files()
            .into_iter()
            .map(|f| (f.path, f.size))
            .collect();
        let approved: BTreeMap<String, u64> =
            m.files.iter().map(|f| (f.path.clone(), f.size)).collect();
        if offered != approved {
            return Err(WaitingError::CopyDamaged);
        }
        let key = decode_key(&r.key).ok_or(WaitingError::File)?;
        Ok((offer, key))
    }

    /// Delivers waiting send `id` to recipient `connection` over `ch` (a known-keys session
    /// with exactly that recipient's key, opened by the caller with [`offer_for`](Self::offer_for)'s
    /// key), and records the result. A recipient that did not answer stays waiting.
    pub async fn deliver(
        &self,
        id: &str,
        connection: &str,
        ch: &mut impl Channel,
        live: &pctwin_move::Live,
        now: u64,
    ) -> Result<Receipt, WaitingError> {
        let (offer, _) = self.offer_for(id, connection, now)?;
        let sent = crate::send_live(ch, offer, live).await;
        let delivery = match &sent {
            Ok(r) => summarise(r, now),
            Err(QuickError::NotAnswered) => Delivery::Waiting,
            Err(e) => Delivery::Failed {
                failed: 0,
                why: e.to_string(),
                at: now,
            },
        };
        let mut m = self.read(id)?;
        if let Some(r) = m.recipients.iter_mut().find(|r| r.connection == connection) {
            r.delivery = delivery;
        }
        self.write(&m)?;
        Ok(sent?)
    }
}

fn decode_key(text: &str) -> Option<[u8; 32]> {
    if text.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, o) in out.iter_mut().enumerate() {
        *o = u8::from_str_radix(text.get(2 * i..2 * i + 2)?, 16).ok()?;
    }
    Some(out)
}

/// One recipient's result from its receipt: delivered only when every file arrived (or was
/// already there, or is there needing care); declined when it declined everything; otherwise
/// failed, saying how many did not arrive.
fn summarise(r: &Receipt, now: u64) -> Delivery {
    let (mut arrived, mut needs, mut declined, mut failed) = (0u64, 0u64, 0u64, 0u64);
    let mut why = String::new();
    for l in &r.lines {
        match &l.outcome {
            Outcome::Arrived { .. } | Outcome::AlreadyThere { .. } => arrived += 1,
            Outcome::NeedsAction { .. } => needs += 1,
            Outcome::Declined => declined += 1,
            Outcome::Failed { why: w, .. } => {
                failed += 1;
                if why.is_empty() {
                    why.clone_from(w);
                }
            }
        }
    }
    if failed > 0 || (declined > 0 && arrived + needs > 0) {
        Delivery::Failed {
            failed: failed + declined,
            why,
            at: now,
        }
    } else if declined > 0 {
        Delivery::Declined { at: now }
    } else {
        Delivery::Delivered {
            arrived,
            needs_action: needs,
            at: now,
        }
    }
}
