//! Waiting sends (Security Design Part T, Quick Send Stage 1): "approve this delivery only".
//!
//! - One approval binds the exact file versions (each file's BLAKE3 fingerprint), each
//!   recipient's own key (a remembered connection's public key), the destination (that recipient
//!   only: where it lands is the recipient's to choose) and a deadline ([`DEFAULT_DAYS`] days,
//!   renewed only when the person asks, [`WaitingStore::renew`]).
//! - The approved versions are copied into PCTwin's own folder on this laptop: **Preparing**
//!   until every copy is made and fingerprinted, then **Waiting**. A send Preparing holds a lease
//!   it renews while it copies ([`LEASE`]): only one whose lease ran out (PCTwin stopped while
//!   copying) is tidied away, never one still being copied. What is sent is that copy, its
//!   fingerprints checked again just before each delivery ([`WaitingStore::offer_for`], on a
//!   thread that may wait, once the recipient was found); a copy that no longer matches is never
//!   sent. A source changed since only says "a newer version exists, review again": it is never
//!   swapped in.
//! - Several recipients: one approval lists each recipient's key; there is no way to add one to
//!   an approval (a new approval is needed); each recipient gets its own session and its own
//!   result, and its own offer ID, so two recipients cannot tell from what they received that
//!   they got the same send.
//! - The copies have a total cap ([`WaitingStore::with_cap`]), need free space first (keeping
//!   [`KEEP_FREE`] spare), show as used space ([`WaitingStore::used_bytes`]), and are deleted at
//!   the deadline ([`WaitingStore::sweep`]) or on cancel. Cancel marks the send cancelled first,
//!   so nothing more is delivered, and removes its key: a copy another program still holds open
//!   is removed by the next tidy, and is unreadable from the moment its key is gone.
//! - The deadline is this laptop's clock; a waiting send delivers only while this laptop is on.
//!
//! Protected storage (Part G): the copies are sealed at rest ([`crate::sealed`]) with a key of
//! their own for each send, kept only in the system's key store (as the connection keys are) and
//! deleted with the send; PCTwin's folder is the person's own (on Mac and Linux made `0700`).
//! What the send holds (names, folders, sizes, fingerprints, where each was read from) is sealed
//! too, in the send's own list; the copies are stored under numbers, so the drive alone shows only
//! how many files wait, their total size, for which connections and until when. On an SSD
//! deleting is not a guaranteed wipe, which is why the key going is what makes a deleted copy
//! unreadable.
//!
//! Every change to a send's record (a delivery's result, a recipient forgotten, a renewal, a
//! cancel, a lease renewed) is made under that send's own lock, read and written together, so one
//! never overwrites another.

use std::collections::{BTreeMap, HashMap};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use pctwin_known::SecretStore;
use pctwin_record::{ItemKind, ItemPath, LaptopId};
use pctwin_transfer::{Channel, KEEP_FREE};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::sealed::{KEY_LEN, LIST_STREAM, SealedOpener, Unsealed};
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
/// How long a send Preparing is held while nothing renews it: a copy still running renews it
/// after every chunk.
pub const LEASE: u64 = 5 * 60;
/// How often a running copy writes its renewed lease.
const RENEW_EVERY: u64 = 30;
const MANIFEST: &str = "waiting.json";
/// The send's sealed list of what it holds.
const LIST: &str = "list.sealed";
const FILES: &str = "files";
/// Largest sealed list read back.
const MOST_LIST_BYTES: u64 = 256 * 1024 * 1024;
const VERSION: u32 = 3;

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
    /// The system's key store could not keep or give the send's key.
    #[error("the system's key store could not be reached")]
    Store,
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
    /// Cancelled: nothing more is delivered; removed as soon as nothing holds it open.
    Cancelled,
}

/// One approved file, as the send's sealed list keeps it (its copy is `files/<n>.sealed`, `n`
/// its place in the list).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaitingFile {
    /// Its path in the send, `/` between folders.
    pub path: String,
    /// Its path in the send, name by name, as offered.
    item: ItemPath,
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
    /// While Preparing: held until then unless the copy renews it.
    lease_until: u64,
    /// How many files, and their approved bytes (the rest is in the sealed list).
    files: u64,
    bytes: u64,
    recipients: Vec<Recipient>,
}

/// What a send holds, sealed with its key.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct List {
    id: String,
    files: Vec<WaitingFile>,
}

/// Each send's lock, by its folder: every read-and-write of its record holds it.
fn send_lock(dir: &Path) -> Arc<Mutex<()>> {
    static LOCKS: OnceLock<Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>> = OnceLock::new();
    let mut all = LOCKS
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    all.entry(dir.to_path_buf()).or_default().clone()
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

/// PCTwin's folder of waiting sends, their keys in `keys`.
pub struct WaitingStore {
    root: PathBuf,
    cap: u64,
    keys: Arc<dyn SecretStore>,
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn modified(meta: &std::fs::Metadata) -> Option<u128> {
    meta.modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_nanos())
}

fn key_name(id: &str) -> String {
    format!("waiting-{id}-key")
}

/// Seconds since 1970, by this laptop's clock.
fn clock() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
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
    /// The waiting sends kept in `root` (PCTwin's own folder for them), with [`DEFAULT_CAP`],
    /// each send's key in `keys` (the system's key store).
    pub fn new(root: &Path, keys: Arc<dyn SecretStore>) -> Self {
        Self {
            root: root.to_path_buf(),
            cap: DEFAULT_CAP,
            keys,
        }
    }

    fn key(&self, id: &str) -> Result<Zeroizing<[u8; KEY_LEN]>, WaitingError> {
        let bytes = self
            .keys
            .get(&key_name(id))
            .map_err(|_| WaitingError::Store)?
            .ok_or(WaitingError::CopyDamaged)?;
        let k: [u8; KEY_LEN] = bytes
            .as_slice()
            .try_into()
            .map_err(|_| WaitingError::CopyDamaged)?;
        Ok(Zeroizing::new(k))
    }

    /// Removes send `id`: its key first (the copy is unreadable from then on), then its folder.
    /// `false` when the folder is still there (something holds it open; the next tidy tries
    /// again).
    fn remove(&self, id: &str) -> bool {
        let Ok(dir) = self.dir(id) else { return false };
        if self.keys.delete(&key_name(id)).is_err() {
            return false;
        }
        match std::fs::remove_dir_all(&dir) {
            Ok(()) => true,
            Err(e) => e.kind() == std::io::ErrorKind::NotFound,
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

    /// Changes send `id`'s record by `change` under the send's lock, read and written together:
    /// `false` from `change` writes nothing. Whether it was written.
    fn update(
        &self,
        id: &str,
        change: impl FnOnce(&mut Manifest) -> bool,
    ) -> Result<bool, WaitingError> {
        let lock = send_lock(&self.dir(id)?);
        let _held = lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut m = self.read(id)?;
        if !change(&mut m) {
            return Ok(false);
        }
        self.write(&m)?;
        Ok(true)
    }

    /// What send `id` holds, from its sealed list.
    fn list_of(&self, id: &str, key: &[u8; KEY_LEN]) -> Result<Vec<WaitingFile>, WaitingError> {
        let bytes = crate::sealed::unseal_bytes(
            &self.dir(id)?.join(LIST),
            key,
            LIST_STREAM,
            MOST_LIST_BYTES,
        )
        .map_err(|_| WaitingError::CopyDamaged)?;
        let list: List = serde_json::from_slice(&bytes).map_err(|_| WaitingError::CopyDamaged)?;
        if list.id != id {
            return Err(WaitingError::CopyDamaged);
        }
        Ok(list.files)
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
            .map(|m| m.bytes)
            .fold(0u64, u64::saturating_add)
    }

    /// Approve this delivery only: the `chosen` files and folders, as they are now, for each of
    /// `recipients` (a connection id and its public key), until `days` days after `now`. Checks
    /// the cap and `free` space (`None`: the drive cannot say) before copying anything; seals and
    /// fingerprints each file (Preparing, its lease renewed as it goes), then marks it Waiting. A
    /// file that changed while it was copied ends it, nothing kept.
    pub fn prepare(
        &self,
        chosen: &[PathBuf],
        recipients: &[(String, [u8; 32])],
        days: u64,
        now: u64,
        free: Option<u64>,
    ) -> Result<WaitingInfo, WaitingError> {
        self.prepare_with_clock(chosen, recipients, days, now, free, &clock)
    }

    /// As [`prepare`](Self::prepare), the lease renewed by `clock` (tests).
    pub fn prepare_with_clock(
        &self,
        chosen: &[PathBuf],
        recipients: &[(String, [u8; 32])],
        days: u64,
        now: u64,
        free: Option<u64>,
        clock: &dyn Fn() -> u64,
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
        let mut recipients_out = Vec::new();
        for (connection, key) in recipients {
            recipients_out.push(Recipient {
                connection: connection.clone(),
                key: hex(key),
                laptop: LaptopId::generate().map_err(QuickError::from)?.to_hex(),
                delivery: Delivery::Waiting,
            });
        }
        // The send's key, kept in the system's store and read back before anything is sealed.
        let key = crate::sealed::new_key().map_err(|_| WaitingError::Store)?;
        self.keys
            .put(&key_name(&id), key.as_ref())
            .map_err(|_| WaitingError::Store)?;
        if self.key(&id).ok().as_deref() != Some(&*key) {
            let _ = self.keys.delete(&key_name(&id));
            return Err(WaitingError::Store);
        }
        let m = Manifest {
            version: VERSION,
            id: id.clone(),
            made_at: now,
            deadline: now.saturating_add(days * DAY),
            state: State::Preparing,
            lease_until: clock().saturating_add(LEASE),
            files: 0,
            bytes: total,
            recipients: recipients_out,
        };
        let copied = (|| {
            make_private_dir(&dir.join(FILES)).map_err(|_| WaitingError::File)?;
            self.write(&m)?;
            let mut files: Vec<WaitingFile> = Vec::new();
            let mut renewed = clock();
            for job in &jobs {
                let Some(item) = items.get(&job.item) else {
                    continue;
                };
                if item.kind != ItemKind::File {
                    continue;
                }
                let stream = u32::try_from(files.len() + 1)
                    .ok()
                    .filter(|n| *n < u32::MAX)
                    .ok_or(WaitingError::Quick(QuickError::TooMany {
                        files: files.len(),
                    }))?;
                let to = copy_path(&dir, files.len());
                let changed = || WaitingError::ChangedWhileCopying(job.path.clone());
                let before = std::fs::metadata(&job.source).map_err(|_| changed())?;
                let mut src = std::fs::File::open(&job.source).map_err(|_| changed())?;
                let mut lease_failed = false;
                let (n, fingerprint) =
                    crate::sealed::seal(&mut src, &to, &key, stream, &mut || {
                        let t = clock();
                        if t.saturating_sub(renewed) >= RENEW_EVERY {
                            renewed = t;
                            let lease = t.saturating_add(LEASE);
                            lease_failed |= self
                                .update(&id, |m| {
                                    m.lease_until = lease;
                                    true
                                })
                                .is_err();
                        }
                    })
                    .map_err(|_| changed())?;
                if lease_failed {
                    return Err(WaitingError::File);
                }
                let after = std::fs::metadata(&job.source).map_err(|_| changed())?;
                if n != before.len() || after.len() != n || modified(&before) != modified(&after) {
                    return Err(changed());
                }
                files.push(WaitingFile {
                    path: job.path.clone(),
                    item: item.path.clone(),
                    size: n,
                    fingerprint,
                    source: job.source.clone(),
                    source_modified: modified(&after),
                });
            }
            // What it holds, sealed once; then Waiting, unless it was cancelled meanwhile.
            let list = serde_json::to_vec(&List {
                id: id.clone(),
                files: files.clone(),
            })
            .map_err(|_| WaitingError::File)?;
            crate::sealed::seal_bytes(&list, &dir.join(LIST), &key, LIST_STREAM)
                .map_err(|_| WaitingError::File)?;
            let (count, bytes) = (
                files.len() as u64,
                files.iter().fold(0u64, |t, f| t.saturating_add(f.size)),
            );
            let ready = self.update(&id, |m| {
                if m.state != State::Preparing {
                    return false;
                }
                m.state = State::Waiting;
                m.files = count;
                m.bytes = bytes;
                true
            })?;
            if !ready {
                return Err(WaitingError::NotFound);
            }
            Ok(())
        })();
        if let Err(e) = copied {
            self.remove(&id);
            return Err(e);
        }
        Ok(self.info(&self.read(&id)?))
    }

    fn info(&self, m: &Manifest) -> WaitingInfo {
        // What changed since needs the sealed list; without the key nothing can be said.
        let newer = self
            .key(&m.id)
            .and_then(|k| self.list_of(&m.id, &k))
            .unwrap_or_default()
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
            files: m.files,
            bytes: m.bytes,
            recipients: m.recipients.clone(),
            newer,
        }
    }

    /// Every waiting send, oldest first (one still Preparing, or cancelled, is not listed; the
    /// tidy, [`sweep`](Self::sweep), removes what should go). Listing never deletes anything.
    pub fn list(&self) -> Vec<WaitingInfo> {
        let mut out: Vec<WaitingInfo> = self
            .ids()
            .iter()
            .filter_map(|id| self.read(id).ok())
            .filter(|m| m.state == State::Waiting)
            .map(|m| self.info(&m))
            .collect();
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
        let deadline = now.saturating_add(days * DAY);
        let renewed = self.update(id, |m| {
            if m.state != State::Waiting {
                return false;
            }
            m.deadline = deadline;
            true
        })?;
        if renewed {
            Ok(())
        } else {
            Err(WaitingError::NotFound)
        }
    }

    /// Cancels a waiting send: marked cancelled first (nothing more is delivered), then its key
    /// and copies are deleted. A copy something still holds open (a delivery the caller is
    /// stopping) is unreadable already and is removed by the next tidy.
    pub fn cancel(&self, id: &str) -> Result<(), WaitingError> {
        self.mark_cancelled(id)?;
        self.remove(id);
        Ok(())
    }

    /// The first step of [`cancel`](Self::cancel): from now nothing of send `id` is offered or
    /// delivered, so a caller can stop a delivery in flight before the copy is deleted.
    pub fn mark_cancelled(&self, id: &str) -> Result<(), WaitingError> {
        let dir = self.dir(id)?;
        let marked = self.update(id, |m| {
            let fresh = m.state != State::Cancelled;
            m.state = State::Cancelled;
            fresh
        });
        match marked {
            Ok(_) => Ok(()),
            Err(_) if dir.exists() => Ok(()),
            Err(e) => Err(e),
        }
    }

    /// Ends every delivery to connection `connection` (it was forgotten): it is taken off each
    /// waiting send; a send left with no recipient is deleted. Returns how many sends changed.
    pub fn forget_recipient(&self, connection: &str) -> usize {
        let mut changed = 0;
        for id in self.ids() {
            let mut nobody = false;
            let took = self.update(&id, |m| {
                let before = m.recipients.len();
                m.recipients.retain(|r| r.connection != connection);
                nobody = m.recipients.is_empty();
                m.recipients.len() != before
            });
            if !matches!(took, Ok(true)) {
                continue;
            }
            changed += 1;
            if nobody {
                let _ = self.cancel(&id);
            }
        }
        changed
    }

    /// The tidy: deletes every waiting send past its deadline at `now`, every cancelled one, and
    /// any left Preparing whose lease ran out (PCTwin stopped while copying; a copy still running
    /// renews its lease, so it is never deleted from under it); returns the ids deleted.
    pub fn sweep(&self, now: u64) -> Vec<String> {
        let mut gone = Vec::new();
        for id in self.ids() {
            let expired = match self.read(&id) {
                Ok(m) => match m.state {
                    State::Waiting => now >= m.deadline,
                    State::Preparing => now >= m.lease_until,
                    State::Cancelled => true,
                },
                // No readable record: a damaged one, or one a copy has only just begun. Only
                // one untouched for a whole lease goes.
                Err(_) => self.dir(&id).ok().is_some_and(|d| untouched_for(&d, LEASE)),
            };
            if expired && self.remove(&id) {
                gone.push(id);
            }
        }
        gone
    }

    /// The copy checked against the approval: every file there, its seal intact and its plain
    /// bytes the approved size and fingerprint, and nothing else in the folder.
    fn verify(
        &self,
        id: &str,
        files: &[WaitingFile],
        key: &[u8; KEY_LEN],
    ) -> Result<(), WaitingError> {
        let dir = self.dir(id)?;
        let mut expected = std::collections::BTreeSet::new();
        for (n, f) in files.iter().enumerate() {
            let at = copy_path(&dir, n);
            let stream = u32::try_from(n + 1).map_err(|_| WaitingError::CopyDamaged)?;
            let fp = Unsealed::open(&at, key, stream, f.size)
                .and_then(|mut u| u.fingerprint())
                .map_err(|_| WaitingError::CopyDamaged)?;
            if fp != f.fingerprint {
                return Err(WaitingError::CopyDamaged);
            }
            expected.insert(at);
        }
        // Nothing added beside the approved copies.
        for e in std::fs::read_dir(dir.join(FILES)).map_err(|_| WaitingError::CopyDamaged)? {
            let e = e.map_err(|_| WaitingError::CopyDamaged)?;
            let t = e.file_type().map_err(|_| WaitingError::CopyDamaged)?;
            if !t.is_file() || !expected.contains(&e.path()) {
                return Err(WaitingError::CopyDamaged);
            }
        }
        Ok(())
    }

    /// Whether send `id` may still be delivered to `connection` at `now` (Waiting, before its
    /// deadline, that recipient still waiting): cheap, read before looking for the recipient.
    pub fn due(&self, id: &str, connection: &str, now: u64) -> bool {
        self.read(id).is_ok_and(|m| {
            m.state == State::Waiting
                && now < m.deadline
                && m.recipients
                    .iter()
                    .any(|r| r.connection == connection && r.delivery == Delivery::Waiting)
        })
    }

    /// The offer of the protected copy for recipient `connection`, checked against the approval
    /// just now: every sealed copy read through and its plain bytes the approved size and
    /// fingerprint. Also the key it may go to. This reads every copied byte, so it runs on a
    /// thread that may wait, after the recipient was found nearby and just before sending.
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
        let key = self.key(id)?;
        let listed = self.list_of(id, &key)?;
        if listed.len() as u64 != m.files {
            return Err(WaitingError::CopyDamaged);
        }
        self.verify(id, &listed, &key)?;
        let laptop = LaptopId::from_hex(&r.laptop).map_err(|_| WaitingError::File)?;
        let dir = self.dir(id)?;
        let mut files = Vec::new();
        let mut opened = Vec::new();
        for (n, f) in listed.iter().enumerate() {
            let at = copy_path(&dir, n);
            let stream = u32::try_from(n + 1).map_err(|_| WaitingError::CopyDamaged)?;
            opened.push((at.clone(), stream, f.size));
            files.push((f.item.clone(), f.size, at));
        }
        let offer = Offer::of_files(laptop, files, Arc::new(SealedOpener::new(key, opened)))?;
        // The offer must list exactly the approved files.
        let offered: BTreeMap<String, u64> = offer
            .files()
            .into_iter()
            .map(|f| (f.path, f.size))
            .collect();
        let approved: BTreeMap<String, u64> =
            listed.iter().map(|f| (f.path.clone(), f.size)).collect();
        if offered != approved {
            return Err(WaitingError::CopyDamaged);
        }
        let peer = decode_key(&r.key).ok_or(WaitingError::File)?;
        Ok((offer, peer))
    }

    /// Delivers `offer` (from [`offer_for`](Self::offer_for)) of waiting send `id` to recipient
    /// `connection` over `ch` (a known-keys session with exactly that recipient's key), and
    /// records the result. A recipient that did not answer stays waiting; a send cancelled or
    /// gone meanwhile records nothing.
    pub async fn deliver(
        &self,
        id: &str,
        connection: &str,
        offer: Offer,
        ch: &mut impl Channel,
        live: &pctwin_move::Live,
        now: u64,
    ) -> Result<Receipt, WaitingError> {
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
        // Read and written under the send's lock: a recipient forgotten or a cancel meanwhile is
        // kept, never written over.
        let recorded = self.update(id, |m| {
            if m.state != State::Waiting {
                return false;
            }
            match m.recipients.iter_mut().find(|r| r.connection == connection) {
                Some(r) => {
                    r.delivery = delivery;
                    true
                }
                None => false,
            }
        });
        if let Err(e) = recorded
            && !matches!(e, WaitingError::NotFound)
        {
            return Err(e);
        }
        Ok(sent?)
    }
}

/// Where the copy of a send's file number `n` is, inside the send's folder `dir`: a number, never
/// its name.
fn copy_path(dir: &Path, n: usize) -> PathBuf {
    dir.join(FILES).join(format!("{n}.sealed"))
}

/// Whether the folder `dir` was last changed at least `secs` ago.
fn untouched_for(dir: &Path, secs: u64) -> bool {
    std::fs::metadata(dir)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.elapsed().ok())
        .is_some_and(|age| age.as_secs() >= secs)
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
