//! The book of remembered connections, and their permissions (Security Design Part T).
//!
//! Each connection is kept on this laptop only (no account, no server): a random id, the name
//! the person gave it, the other laptop's public key, when it was made and last used, and the
//! fingerprint of the hardware it was remembered on. Its private key and the pair's secret live
//! only in the system's key store ([`crate::store`]), under names made from the id.
//!
//! **The three permissions are stored separately and each can be withdrawn on its own:**
//! 1. *Remember this connection*: the connection itself ([`Book::remember`], [`Book::forget`]).
//! 2. *Accept these particular files*: asked every time, for each send. Nothing is stored: a
//!    yes covers that one send and is gone when it ends, so it can never become a standing yes.
//! 3. *Allow future sends into this folder* ([`FolderGrant`]): off at first, for one connection
//!    and one folder, ending on its own after [`GRANT_LIFETIME`] unless renewed, delivering into
//!    a PCTwin-named folder inside the chosen one, with a daily size and count cap, and telling the
//!    person on the first delivery after a quiet spell ([`QUIET_SPELL`]).
//!
//! "Folder for next time" ([`NextFolder`]) is kept apart from permission 3: a different type in a
//! different field, with no way to turn one into the other. It only chooses the folder shown when
//! files arrive; the person still says Save.
//!
//! A connection unused for [`UNUSED_FOR`] expires and is forgotten ([`Book::expire`]).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use hmac::{Hmac, KeyInit, Mac};
use pctwin_pairing::known::StaticKeys;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use zeroize::Zeroizing;

use crate::folders::{FolderProblem, KnownFolder};
use crate::store::{SecretStore, StoreError};
use crate::swap::Pending;
use crate::{hex, unhex};

/// Seconds in a day.
pub const DAY_SECONDS: u64 = 24 * 60 * 60;
/// A connection unused this long expires.
pub const UNUSED_FOR: u64 = 90 * DAY_SECONDS;
/// Permission 3 ends on its own this long after it was given or last renewed.
pub const GRANT_LIFETIME: u64 = 30 * DAY_SECONDS;
/// After this long with no delivery under permission 3, the next one is told to the person.
pub const QUIET_SPELL: u64 = 7 * DAY_SECONDS;
/// Permission 3's caps, unless the person set others: bytes and deliveries per day.
pub const DEFAULT_DAILY_BYTES: u64 = 5 * 1024 * 1024 * 1024;
pub const DEFAULT_DAILY_SENDS: u32 = 20;
/// How long each announced token lasts.
pub const TOKEN_PERIOD: u64 = 15 * 60;
/// The name of the folder permission 3 delivers into, inside the chosen one.
pub const DELIVERY_FOLDER: &str = "PCTwin Quick Send";
/// Longest name a person may give a connection, in characters.
pub const MAX_LABEL_CHARS: usize = 40;
/// Most connections remembered.
pub const MAX_CONNECTIONS: usize = 200;
const TOKEN_LABEL: &[u8] = b"pctwin/v1/known-token";
const BOOK_VERSION: u32 = 1;

/// Why the book could not do what was asked.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BookError {
    #[error("no such connection")]
    NotFound,
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("the list of remembered connections could not be read or saved")]
    File,
    /// The connection was remembered on other hardware: this is a restored backup or a copied
    /// drive. Its keys are not used; pair again.
    #[error("this laptop was restored or copied since this connection was remembered; pair again")]
    Restored,
    #[error("the name is empty or too long")]
    BadLabel,
    #[error("too many remembered connections")]
    Full,
}

/// Why permission 3 does not let a delivery in now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, thiserror::Error)]
#[serde(rename_all = "kebab-case")]
pub enum GrantProblem {
    #[error("no folder is allowed for this connection")]
    None,
    #[error("the permission ended; renew it to allow sends again")]
    Expired,
    #[error("today's limit for this connection is reached")]
    OverCap,
    /// The allowed folder cannot be used now (moved, swapped, or a refused place): nothing was
    /// counted, and the person is asked instead.
    #[error("the allowed folder cannot be used now")]
    Unusable,
}

/// Whether this laptop is the one a connection was remembered on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Device {
    /// The same hardware as when it was remembered.
    Same,
    /// Other hardware: a restored backup or a copied drive. Its keys are not used.
    Restored,
    /// The hardware gave no identity that tells laptops apart (none, all zeros, a maker's example
    /// value), then or now: PCTwin cannot tell, and says so plainly; never "same device".
    CannotTell,
}

/// Permission 3: future sends from one connection into one folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FolderGrant {
    pub folder: KnownFolder,
    pub given_at: u64,
    pub ends_at: u64,
    pub daily_bytes: u64,
    pub daily_sends: u32,
    /// The day (days since 1970) the counts below are for.
    day: u64,
    bytes_today: u64,
    sends_today: u32,
    last_delivery: Option<u64>,
}

/// "Folder for next time": only the folder shown when files from this connection arrive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NextFolder(pub KnownFolder);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Connection {
    id: String,
    label: String,
    peer_public: String,
    made_at: u64,
    last_used: u64,
    machine: Option<String>,
    #[serde(default)]
    grant: Option<FolderGrant>,
    #[serde(default)]
    next_folder: Option<NextFolder>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct BookFile {
    version: u32,
    connections: Vec<Connection>,
}

/// A remembered connection as the screens list it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionInfo {
    pub id: String,
    pub label: String,
    pub made_at: u64,
    pub last_used: u64,
    /// When it expires unless used before.
    pub expires_at: u64,
    /// Whether this laptop is the one it was remembered on.
    pub device: Device,
    pub grant: Option<GrantInfo>,
    pub next_folder: Option<PathBuf>,
}

/// Permission 3 as the screens show it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantInfo {
    pub folder: PathBuf,
    pub ends_at: u64,
    pub ended: bool,
    pub daily_bytes: u64,
    pub daily_sends: u32,
}

/// What permission 3 lets in for one delivery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Admitted {
    pub folder: KnownFolder,
    /// The first delivery after a quiet spell: tell the person.
    pub tell: bool,
}

/// The remembered connections, kept in one file at `path`, secrets in `store`.
pub struct Book {
    path: PathBuf,
    store: Arc<dyn SecretStore>,
    machine: Option<String>,
    file: BookFile,
    /// The tokens of one window, worked out once (see [`Tokens`]).
    tokens: Option<Tokens>,
}

/// Every connection's tokens around one token window, in memory only: worked out from the key
/// store once per window (or when the connections change), so a hello from any device nearby is
/// a lookup here and never a read of the key store.
struct Tokens {
    window: u64,
    /// This laptop's own tokens for the window before, this one and the next: the connection
    /// each names.
    hints: HashMap<[u8; 8], String>,
    /// What this laptop announces in this window, per connection.
    announced: Vec<(String, [u8; 8])>,
}

fn key_name(id: &str) -> String {
    format!("known-{id}-key")
}

fn pair_name(id: &str) -> String {
    format!("known-{id}-pair")
}

fn label_ok(label: &str) -> Result<String, BookError> {
    let label = label.trim();
    let n = label.chars().count();
    if n == 0 || n > MAX_LABEL_CHARS || label.chars().any(char::is_control) {
        return Err(BookError::BadLabel);
    }
    Ok(label.to_string())
}

/// The token announced for one side of a pair in `window`: HMAC-SHA256 of the pair's secret over
/// the side and the window, its first eight bytes. Only a laptop holding the pair's secret can
/// work it out or recognise it, and it changes every [`TOKEN_PERIOD`].
pub fn token(pair_secret: &[u8; 32], low_side: bool, window: u64) -> [u8; 8] {
    let mut mac =
        <Hmac<Sha256> as KeyInit>::new_from_slice(pair_secret).expect("HMAC takes any key length");
    mac.update(TOKEN_LABEL);
    mac.update(&[u8::from(low_side)]);
    mac.update(&window.to_be_bytes());
    let out = mac.finalize().into_bytes();
    let mut t = [0u8; 8];
    t.copy_from_slice(&out[..8]);
    t
}

impl Book {
    /// The book at `path` (empty when there is none yet), with secrets in `store`, on the
    /// hardware whose fingerprint is `machine` ([`crate::machine::this_machine`]).
    pub fn open(
        path: &Path,
        store: Arc<dyn SecretStore>,
        machine: Option<String>,
    ) -> Result<Self, BookError> {
        let file = match std::fs::read(path) {
            Ok(bytes) => {
                let f: BookFile = serde_json::from_slice(&bytes).map_err(|_| BookError::File)?;
                if f.version != BOOK_VERSION {
                    return Err(BookError::File);
                }
                f
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => BookFile {
                version: BOOK_VERSION,
                connections: Vec::new(),
            },
            Err(_) => return Err(BookError::File),
        };
        Ok(Self {
            path: path.to_path_buf(),
            store,
            machine,
            file,
            tokens: None,
        })
    }

    /// Written to a new file beside it, flushed, then moved over it in one step.
    fn save(&self) -> Result<(), BookError> {
        use std::io::Write;
        let bytes = serde_json::to_vec_pretty(&self.file).map_err(|_| BookError::File)?;
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(|_| BookError::File)?;
        }
        let part = self.path.with_extension("part");
        let mut f = std::fs::File::create(&part).map_err(|_| BookError::File)?;
        f.write_all(&bytes)
            .and_then(|()| f.sync_all())
            .map_err(|_| BookError::File)?;
        drop(f);
        std::fs::rename(&part, &self.path).map_err(|_| BookError::File)
    }

    fn find(&self, id: &str) -> Result<&Connection, BookError> {
        self.file
            .connections
            .iter()
            .find(|c| c.id == id)
            .ok_or(BookError::NotFound)
    }

    fn find_mut(&mut self, id: &str) -> Result<&mut Connection, BookError> {
        self.file
            .connections
            .iter_mut()
            .find(|c| c.id == id)
            .ok_or(BookError::NotFound)
    }

    fn device(&self, c: &Connection) -> Device {
        match (&c.machine, &self.machine) {
            (Some(then), Some(now)) if then == now => Device::Same,
            // Remembered on hardware that could tell, and this one now cannot or differs: never
            // taken for the same laptop (a flaky hardware query asks to pair again: accepted).
            (Some(_), _) => Device::Restored,
            // Remembered where the system could not tell laptops apart.
            (None, _) => Device::CannotTell,
        }
    }

    /// Remember this connection (permission 1), named `label`, at time `now` (seconds since
    /// 1970). The secrets go to the store first and are read back; then the connection is written
    /// to the book. Only once it is kept is the earlier connection it `replaces` (the same other
    /// laptop paired again: its keys are fresh, so only the person can say) forgotten, its keys
    /// and every permission; if that cannot be done the new one is taken back, so nothing is half
    /// done. `pending` is only read: on any error the caller still holds it. Returns its id.
    pub fn remember(
        &mut self,
        pending: &Pending,
        label: &str,
        now: u64,
        replaces: Option<&str>,
    ) -> Result<String, BookError> {
        let label = label_ok(label)?;
        if let Some(old) = replaces {
            self.find(old)?;
        }
        let kept = self.file.connections.len() - usize::from(replaces.is_some());
        if kept >= MAX_CONNECTIONS {
            return Err(BookError::Full);
        }
        let peer = hex(&pending.peer_public);
        let mut raw = [0u8; 8];
        getrandom::fill(&mut raw).map_err(|_| BookError::Store(StoreError::Unavailable))?;
        let id = hex(&raw);
        self.store
            .put(&key_name(&id), pending.keys.private().as_ref())?;
        self.store
            .put(&pair_name(&id), pending.pair_secret.as_ref())?;
        let back = self.store.get(&key_name(&id))?;
        if back.as_deref().map(Vec::as_slice) != Some(pending.keys.private().as_slice()) {
            let _ = self.store.delete(&key_name(&id));
            let _ = self.store.delete(&pair_name(&id));
            return Err(BookError::Store(StoreError::Unavailable));
        }
        self.file.connections.push(Connection {
            id: id.clone(),
            label,
            peer_public: peer.clone(),
            made_at: now,
            last_used: now,
            machine: self.machine.clone(),
            grant: None,
            next_folder: None,
        });
        self.tokens = None;
        if let Err(e) = self.save() {
            self.file.connections.retain(|c| c.id != id);
            let _ = self.store.delete(&key_name(&id));
            let _ = self.store.delete(&pair_name(&id));
            return Err(e);
        }
        // Now the earlier connection goes: the one named, and any with this very key.
        let mut old: Vec<String> = self
            .file
            .connections
            .iter()
            .filter(|c| c.id != id && c.peer_public == peer)
            .map(|c| c.id.clone())
            .collect();
        if let Some(r) = replaces
            && !old.iter().any(|o| o == r)
        {
            old.push(r.to_string());
        }
        for o in old {
            if let Err(e) = self.forget(&o) {
                // Taken back, so the person can try again with nothing half done.
                self.file.connections.retain(|c| c.id != id);
                self.tokens = None;
                let _ = self.save();
                let _ = self.store.delete(&key_name(&id));
                let _ = self.store.delete(&pair_name(&id));
                return Err(e);
            }
        }
        Ok(id)
    }

    /// Permission 3's folder for connection `id`, when it is given and has not ended at `now`:
    /// what to open (outside any lock) before [`admit_with`](Self::admit_with) confirms it.
    pub fn grant_folder(&self, id: &str, now: u64) -> Option<KnownFolder> {
        let g = self.find(id).ok()?.grant.as_ref()?;
        (now < g.ends_at).then(|| g.folder.clone())
    }

    /// The connections with the same name as `label` (ignoring case and spaces around it): the
    /// earlier connection a person pairing the same laptop again most likely means to replace.
    pub fn same_label(&self, label: &str) -> Vec<String> {
        let label = label.trim().to_lowercase();
        self.file
            .connections
            .iter()
            .filter(|c| c.label.trim().to_lowercase() == label)
            .map(|c| c.id.clone())
            .collect()
    }

    /// Whether connection `id` is (still) remembered.
    pub fn has(&self, id: &str) -> bool {
        self.find(id).is_ok()
    }

    /// Forget this connection: its key and pair secret leave the store first, then the
    /// connection and every permission with it leave the book. When the store cannot be reached,
    /// nothing is claimed forgotten.
    pub fn forget(&mut self, id: &str) -> Result<(), BookError> {
        self.find(id)?;
        self.store.delete(&key_name(id))?;
        self.store.delete(&pair_name(id))?;
        self.file.connections.retain(|c| c.id != id);
        self.tokens = None;
        self.save()
    }

    /// Forgets every connection unused for [`UNUSED_FOR`] at `now`; returns their ids (the
    /// caller ends their unfinished deliveries).
    pub fn expire(&mut self, now: u64) -> Result<Vec<String>, BookError> {
        let old: Vec<String> = self
            .file
            .connections
            .iter()
            .filter(|c| now.saturating_sub(c.last_used) >= UNUSED_FOR)
            .map(|c| c.id.clone())
            .collect();
        for id in &old {
            self.forget(id)?;
        }
        Ok(old)
    }

    /// The connections at `now`, newest use first, each permission 3 marked ended when it is.
    pub fn list(&self, now: u64) -> Vec<ConnectionInfo> {
        let mut out: Vec<ConnectionInfo> = self
            .file
            .connections
            .iter()
            .map(|c| ConnectionInfo {
                id: c.id.clone(),
                label: c.label.clone(),
                made_at: c.made_at,
                last_used: c.last_used,
                expires_at: c.last_used.saturating_add(UNUSED_FOR),
                device: self.device(c),
                grant: c.grant.as_ref().map(|g| GrantInfo {
                    folder: g.folder.path.clone(),
                    ends_at: g.ends_at,
                    ended: now >= g.ends_at,
                    daily_bytes: g.daily_bytes,
                    daily_sends: g.daily_sends,
                }),
                next_folder: c.next_folder.as_ref().map(|n| n.0.path.clone()),
            })
            .collect();
        out.sort_by(|a, b| b.last_used.cmp(&a.last_used).then(a.id.cmp(&b.id)));
        out
    }

    /// Renames a connection (the name is the person's own, shown as typed, and proves nothing).
    pub fn rename(&mut self, id: &str, label: &str) -> Result<(), BookError> {
        let label = label_ok(label)?;
        self.find_mut(id)?.label = label;
        self.save()
    }

    /// The other laptop's remembered public key.
    pub fn peer_public(&self, id: &str) -> Result<[u8; 32], BookError> {
        unhex(&self.find(id)?.peer_public).ok_or(BookError::File)
    }

    fn check_device(&self, id: &str) -> Result<&Connection, BookError> {
        let c = self.find(id)?;
        if self.device(c) == Device::Restored {
            return Err(BookError::Restored);
        }
        Ok(c)
    }

    /// This laptop's keys for a connection, read from the store for the session about to run
    /// (never kept). Refused on a restored or copied laptop.
    pub fn keys_for(&self, id: &str) -> Result<StaticKeys, BookError> {
        self.check_device(id)?;
        let bytes = self.store.get(&key_name(id))?.ok_or(StoreError::Damaged)?;
        let private: [u8; 32] = bytes
            .as_slice()
            .try_into()
            .map_err(|_| StoreError::Damaged)?;
        Ok(StaticKeys::from_private(Zeroizing::new(private)))
    }

    fn pair_secret(&self, id: &str) -> Result<Zeroizing<[u8; 32]>, BookError> {
        self.check_device(id)?;
        let bytes = self.store.get(&pair_name(id))?.ok_or(StoreError::Damaged)?;
        let s: [u8; 32] = bytes
            .as_slice()
            .try_into()
            .map_err(|_| StoreError::Damaged)?;
        Ok(Zeroizing::new(s))
    }

    /// Whether this laptop is the "low" side of the pair (its public key is the lower).
    fn low_side(&self, id: &str) -> Result<bool, BookError> {
        let own = self.keys_for(id)?.public();
        Ok(own < self.peer_public(id)?)
    }

    /// The token this laptop announces for each connection now, one per connection, each in
    /// its own announcement. The tokens themselves cannot be linked to each other, but every
    /// announcement from this laptop carries the same addresses and port, so anyone on the
    /// network can group them by where they come from (and count this laptop's connections);
    /// they say nothing about who the connections are. Connections whose keys cannot be read
    /// are left out.
    pub fn announced(&mut self, now: u64) -> Vec<(String, [u8; 8])> {
        self.tokens_at(now).announced.clone()
    }

    /// The tokens of `now`'s window, worked out again only when the window or the connections
    /// changed.
    fn tokens_at(&mut self, now: u64) -> &Tokens {
        let window = now / TOKEN_PERIOD;
        if let Some(t) = self.tokens.take_if(|t| t.window == window) {
            return self.tokens.insert(t);
        }
        let mut hints = HashMap::new();
        let mut announced = Vec::new();
        for c in &self.file.connections {
            let (Ok(secret), Ok(low)) = (self.pair_secret(&c.id), self.low_side(&c.id)) else {
                continue;
            };
            for w in [window.saturating_sub(1), window, window + 1] {
                hints.insert(token(&secret, low, w), c.id.clone());
            }
            announced.push((c.id.clone(), token(&secret, low, window)));
        }
        self.tokens.insert(Tokens {
            window,
            hints,
            announced,
        })
    }

    /// The tokens the other laptop of connection `id` announces around `now` (the window before,
    /// this one and the next, for clocks a little apart): what to look for nearby.
    pub fn peer_tokens(&self, id: &str, now: u64) -> Result<Vec<[u8; 8]>, BookError> {
        let secret = self.pair_secret(id)?;
        let low = self.low_side(id)?;
        let w = now / TOKEN_PERIOD;
        Ok([w.saturating_sub(1), w, w + 1]
            .into_iter()
            .map(|w| token(&secret, !low, w))
            .collect())
    }

    /// Which connection a laptop that connected here means by `hint` (a token this laptop
    /// announced around `now`). `None`: nobody this laptop remembers. A lookup in memory: the key
    /// store is read at most once per token window, however many devices say hello.
    pub fn lookup(&mut self, hint: &[u8; 8], now: u64) -> Option<String> {
        self.tokens_at(now).hints.get(hint).cloned()
    }

    /// The connection was used at `now` (a session with it finished its handshake).
    pub fn touch(&mut self, id: &str, now: u64) -> Result<(), BookError> {
        let c = self.find_mut(id)?;
        c.last_used = c.last_used.max(now);
        self.save()
    }

    /// Permission 3: future sends from connection `id` into `folder` (already chosen and checked,
    /// [`KnownFolder::choose`]), from `now` for [`GRANT_LIFETIME`], with the default caps. Any
    /// earlier folder for this connection is replaced.
    pub fn allow_folder(
        &mut self,
        id: &str,
        folder: KnownFolder,
        now: u64,
    ) -> Result<(), BookError> {
        self.find_mut(id)?.grant = Some(FolderGrant {
            folder,
            given_at: now,
            ends_at: now.saturating_add(GRANT_LIFETIME),
            daily_bytes: DEFAULT_DAILY_BYTES,
            daily_sends: DEFAULT_DAILY_SENDS,
            day: now / DAY_SECONDS,
            bytes_today: 0,
            sends_today: 0,
            last_delivery: None,
        });
        self.save()
    }

    /// Withdraws permission 3 for connection `id` (the connection and its "folder for next time"
    /// stay).
    pub fn revoke_folder(&mut self, id: &str) -> Result<(), BookError> {
        self.find_mut(id)?.grant = None;
        self.save()
    }

    /// Renews permission 3 from `now` for another [`GRANT_LIFETIME`].
    pub fn renew_folder(&mut self, id: &str, now: u64) -> Result<(), BookError> {
        let g = self
            .find_mut(id)?
            .grant
            .as_mut()
            .ok_or(BookError::NotFound)?;
        g.ends_at = now.saturating_add(GRANT_LIFETIME);
        self.save()
    }

    /// Whether permission 3 lets a delivery of `bytes` from connection `id` in at `now`: within
    /// its time and today's caps, and its folder opened by `open` (rechecked for this delivery).
    /// Only when all of that holds is the delivery counted against today's caps and the quiet
    /// spell ended; when the folder cannot be used nothing is counted and the person is asked,
    /// so the first delivery after a quiet spell is still told.
    pub fn admit_with<T>(
        &mut self,
        id: &str,
        bytes: u64,
        now: u64,
        open: impl FnOnce(&KnownFolder) -> Result<T, FolderProblem>,
    ) -> Result<(Admitted, T), GrantProblem> {
        let c = self.find_mut(id).map_err(|_| GrantProblem::None)?;
        let g = c.grant.as_mut().ok_or(GrantProblem::None)?;
        if now >= g.ends_at {
            return Err(GrantProblem::Expired);
        }
        let today = now / DAY_SECONDS;
        let (bytes_today, sends_today) = if g.day == today {
            (g.bytes_today, g.sends_today)
        } else {
            (0, 0)
        };
        if sends_today >= g.daily_sends || bytes_today.saturating_add(bytes) > g.daily_bytes {
            return Err(GrantProblem::OverCap);
        }
        let opened = open(&g.folder).map_err(|_| GrantProblem::Unusable)?;
        g.day = today;
        g.sends_today = sends_today + 1;
        g.bytes_today = bytes_today.saturating_add(bytes);
        let tell = g
            .last_delivery
            .is_none_or(|last| now.saturating_sub(last) >= QUIET_SPELL);
        g.last_delivery = Some(now);
        let folder = g.folder.clone();
        self.save().map_err(|_| GrantProblem::None)?;
        Ok((Admitted { folder, tell }, opened))
    }

    /// "Use this folder for this connection next time".
    pub fn set_next_folder(&mut self, id: &str, folder: NextFolder) -> Result<(), BookError> {
        self.find_mut(id)?.next_folder = Some(folder);
        self.save()
    }

    pub fn clear_next_folder(&mut self, id: &str) -> Result<(), BookError> {
        self.find_mut(id)?.next_folder = None;
        self.save()
    }

    /// The folder shown when files from connection `id` arrive, if one was chosen for next time.
    pub fn next_folder(&self, id: &str) -> Result<Option<NextFolder>, BookError> {
        Ok(self.find(id)?.next_folder.clone())
    }

    /// The name the person gave connection `id`.
    pub fn label(&self, id: &str) -> Result<String, BookError> {
        Ok(self.find(id)?.label.clone())
    }
}
