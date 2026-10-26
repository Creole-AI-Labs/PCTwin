//! The key that seals each effect record (security review finding 3, 26 October 2026).
//!
//! Every effect record, and every record of undoing one, carries a keyed BLAKE3 tag over its
//! number and its whole content. Undo acts only on a record whose tag is right: a record changed
//! by hand (another app named, removal switched on) is reported damaged and never drives undo.
//!
//! The key is never kept in the journal file, so editing that file cannot make a right tag. It is
//! kept, one per journal, where the system keeps a person's secrets:
//!
//! - **Windows**: the person's Credential Manager (a local, not roaming, generic credential),
//!   which Windows encrypts with the person's own data-protection key (DPAPI).
//! - **macOS**: the person's login Keychain.
//! - **Linux**: a file only the person can read (`~/.config/pctwin/journal-keys/`, folder `0700`,
//!   file `0600`, both checked to be the person's own). This is weaker: any program running as the
//!   person can read it, as it can read the journal itself. It stops a record edited by hand or by
//!   a program that does not know of the key, not a program written against PCTwin.
//!
//! A key is made only for a journal that holds no effect yet. If a journal that does hold effects
//! cannot find its key (the credential was removed, the profile restored without it), nothing is
//! erased and no new key is made over it (acceptance case T23-007, "await the key; do not erase
//! the journal"): its effects read as damaged, never drive undo, and no new change is recorded,
//! until the key is back.
//!
//! What this is not: a wall against code inside PCTwin, which holds the key (the role walls are
//! the recorders and grants), nor against a program running as the person that reads the key the
//! same way PCTwin does.

/// The key that seals one journal's effect records. Never printed.
#[derive(Clone)]
pub struct TagKey([u8; 32]);

impl std::fmt::Debug for TagKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TagKey(redacted)")
    }
}

impl TagKey {
    /// A key from its bytes (tests, and a journal opened with a key kept elsewhere).
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// A new random key from the system's random number source.
    pub(crate) fn random() -> Option<Self> {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).ok()?;
        Some(Self(bytes))
    }

    /// The tag of one record: what kind of record (`table`), its number, its bytes.
    pub(crate) fn tag(&self, table: &str, id: u64, body: &[u8]) -> [u8; 32] {
        let mut h = blake3::Hasher::new_keyed(&self.0);
        h.update(&(table.len() as u64).to_be_bytes());
        h.update(table.as_bytes());
        h.update(&id.to_be_bytes());
        h.update(body);
        *h.finalize().as_bytes()
    }

    /// Whether `tag` is the tag of this record (compared in constant time).
    pub(crate) fn verify(&self, table: &str, id: u64, body: &[u8], tag: &[u8; 32]) -> bool {
        blake3::Hash::from_bytes(self.tag(table, id, body)) == blake3::Hash::from_bytes(*tag)
    }
}

/// A journal's record key: the one it was opened with, or, once found, the system's.
pub(crate) struct Held {
    pub(crate) given: Option<TagKey>,
    /// Where the key is kept when none was given: the system's store (a test's in memory).
    pub(crate) store: std::sync::Arc<dyn KeyStore + Send + Sync>,
    pub(crate) held: std::sync::Mutex<Option<TagKey>>,
}

impl Held {
    pub(crate) fn system() -> Self {
        Self {
            given: None,
            store: std::sync::Arc::new(SystemStore),
            held: std::sync::Mutex::new(None),
        }
    }

    pub(crate) fn given(key: TagKey) -> Self {
        Self {
            given: Some(key),
            ..Self::system()
        }
    }
}

/// Why the key could not be had.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyError {
    /// The journal holds effects and their key is not where it was kept: wait for it.
    Missing,
    /// The system's key store could not be read or written now.
    Unavailable,
}

/// Where the system keeps a journal's key, by the journal's own number.
pub(crate) trait KeyStore {
    fn get(&self, journal_id: u64) -> Result<Option<TagKey>, KeyError>;
    fn put(&self, journal_id: u64, key: &TagKey) -> Result<(), KeyError>;
}

/// The key of journal `journal_id` from `store`: the one kept, or, only when `may_make` (the
/// journal holds no effect yet), a new one, kept first and read back.
pub(crate) fn key_for(
    store: &dyn KeyStore,
    journal_id: u64,
    may_make: bool,
) -> Result<TagKey, KeyError> {
    if let Some(key) = store.get(journal_id)? {
        return Ok(key);
    }
    if !may_make {
        return Err(KeyError::Missing);
    }
    let key = TagKey::random().ok_or(KeyError::Unavailable)?;
    store.put(journal_id, &key)?;
    match store.get(journal_id)? {
        Some(back) if back.0 == key.0 => Ok(key),
        _ => Err(KeyError::Unavailable),
    }
}

fn entry_name(journal_id: u64) -> String {
    format!("journal-{journal_id:016x}")
}

/// The system's own key store for this person.
pub(crate) struct SystemStore;

/// The service name the key is kept under.
#[cfg(any(windows, target_os = "macos"))]
const SERVICE: &str = "PCTwin record key";

#[cfg(any(windows, target_os = "macos"))]
impl SystemStore {
    fn entry(journal_id: u64) -> Result<keyring_core::Entry, KeyError> {
        #[cfg(windows)]
        let store =
            windows_native_keyring_store::Store::new().map_err(|_| KeyError::Unavailable)?;
        #[cfg(target_os = "macos")]
        let store = apple_native_keyring_store::keychain::Store::new()
            .map_err(|_| KeyError::Unavailable)?;
        let mut modifiers = std::collections::HashMap::new();
        #[cfg(windows)]
        modifiers.insert("persistence", "Local");
        keyring_core::api::CredentialStoreApi::build(
            store.as_ref(),
            SERVICE,
            &entry_name(journal_id),
            Some(&modifiers),
        )
        .map_err(|_| KeyError::Unavailable)
    }
}

#[cfg(any(windows, target_os = "macos"))]
impl KeyStore for SystemStore {
    fn get(&self, journal_id: u64) -> Result<Option<TagKey>, KeyError> {
        match Self::entry(journal_id)?.get_secret() {
            Ok(bytes) => <[u8; 32]>::try_from(bytes.as_slice())
                .map(|b| Some(TagKey(b)))
                .map_err(|_| KeyError::Missing),
            Err(keyring_core::Error::NoEntry) => Ok(None),
            Err(_) => Err(KeyError::Unavailable),
        }
    }

    fn put(&self, journal_id: u64, key: &TagKey) -> Result<(), KeyError> {
        Self::entry(journal_id)?
            .set_secret(&key.0)
            .map_err(|_| KeyError::Unavailable)
    }
}

#[cfg(target_os = "linux")]
impl SystemStore {
    /// `$XDG_CONFIG_HOME/pctwin/journal-keys` (or `~/.config/...`), made `0700` if missing, and
    /// checked to be the person's own folder that nobody else can open.
    fn folder() -> Result<std::path::PathBuf, KeyError> {
        use std::os::unix::fs::{DirBuilderExt, MetadataExt};
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .filter(|v| !v.is_empty())
            .map(std::path::PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| std::path::Path::new(&h).join(".config")))
            .ok_or(KeyError::Unavailable)?;
        let dir = base.join("pctwin").join("journal-keys");
        match std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&dir)
        {
            Ok(()) => {}
            Err(_) => return Err(KeyError::Unavailable),
        }
        let meta = std::fs::symlink_metadata(&dir).map_err(|_| KeyError::Unavailable)?;
        let mine = meta.uid() == rustix::process::geteuid().as_raw();
        if !meta.is_dir() || !mine || meta.mode() & 0o077 != 0 {
            return Err(KeyError::Unavailable);
        }
        Ok(dir)
    }
}

#[cfg(target_os = "linux")]
impl KeyStore for SystemStore {
    fn get(&self, journal_id: u64) -> Result<Option<TagKey>, KeyError> {
        use std::io::Read;
        use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
        let path = Self::folder()?.join(entry_name(journal_id));
        let mut file = match std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
            .open(&path)
        {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(KeyError::Unavailable),
        };
        let meta = file.metadata().map_err(|_| KeyError::Unavailable)?;
        if !meta.is_file()
            || meta.uid() != rustix::process::geteuid().as_raw()
            || meta.mode() & 0o077 != 0
        {
            return Err(KeyError::Unavailable);
        }
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|_| KeyError::Unavailable)?;
        <[u8; 32]>::try_from(bytes.as_slice())
            .map(|b| Some(TagKey(b)))
            .map_err(|_| KeyError::Missing)
    }

    fn put(&self, journal_id: u64, key: &TagKey) -> Result<(), KeyError> {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let path = Self::folder()?.join(entry_name(journal_id));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .map_err(|_| KeyError::Unavailable)?;
        file.write_all(&key.0)
            .and_then(|()| file.sync_all())
            .map_err(|_| KeyError::Unavailable)
    }
}

/// Other systems keep no key: effects cannot be recorded there.
#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
impl KeyStore for SystemStore {
    fn get(&self, _: u64) -> Result<Option<TagKey>, KeyError> {
        Err(KeyError::Unavailable)
    }
    fn put(&self, _: u64, _: &TagKey) -> Result<(), KeyError> {
        Err(KeyError::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct Memory(Mutex<HashMap<u64, TagKey>>, Mutex<usize>);

    impl KeyStore for Memory {
        fn get(&self, id: u64) -> Result<Option<TagKey>, KeyError> {
            Ok(self.0.lock().unwrap().get(&id).cloned())
        }
        fn put(&self, id: u64, key: &TagKey) -> Result<(), KeyError> {
            *self.1.lock().unwrap() += 1;
            self.0.lock().unwrap().insert(id, key.clone());
            Ok(())
        }
    }

    /// T23-007, through the journal: a journal holding effects whose key was removed from the
    /// store makes no new key; its effects are not read, nothing new is recorded, nothing is
    /// erased; with the key back, all is as it was.
    #[test]
    fn a_journal_with_effects_waits_for_its_key() {
        use crate::{Journal, JournalError};
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("j.redb");
        let store = Arc::new(Memory::default());
        let open = |store: &Arc<Memory>| {
            let mut j = Journal::open(&path).unwrap();
            j.record_key = Held {
                given: None,
                store: store.clone(),
                held: Mutex::new(None),
            };
            j
        };
        let keys = pctwin_grant::RoleKeys::mint().unwrap();
        let j = open(&store);
        let planned = crate::PlannedEffect {
            scope: crate::Scope([1; 16]),
            item: pctwin_record::ItemId::from_hex(&"ab".repeat(16)).unwrap(),
            recipe: pctwin_recipe::RecipeRef {
                id: pctwin_recipe::RecipeId::new("x").unwrap(),
                version: 1,
            },
            step: 0,
            actor: crate::Actor {
                acting_account: "1".into(),
                for_account: "1".into(),
                permission: crate::Permission::OwnFolders,
            },
        };
        let intent = crate::InstallIntent::Licence {
            identity: "a licence".into(),
        };
        j.installs(&keys.install).intend(&planned, &intent).unwrap();
        assert_eq!(*store.1.lock().unwrap(), 1, "made for the empty journal");
        drop(j);
        let id = *store.0.lock().unwrap().keys().next().unwrap();
        let key = store.0.lock().unwrap().remove(&id).unwrap();
        let j = open(&store);
        assert!(matches!(j.effects(), Err(JournalError::RecordKeyMissing)));
        assert!(matches!(
            j.installs(&keys.install).intend(&planned, &intent),
            Err(JournalError::RecordKeyMissing)
        ));
        assert_eq!(*store.1.lock().unwrap(), 1, "no new key made over it");
        drop(j);
        store.0.lock().unwrap().insert(id, key);
        let j = open(&store);
        assert_eq!(j.effects().unwrap().len(), 1, "nothing was erased");
    }

    #[test]
    fn a_key_is_made_once_and_only_for_a_journal_with_no_effects() {
        let m = Memory::default();
        // A journal holding effects whose key is gone: waits for it, never makes a new one.
        assert_eq!(key_for(&m, 7, false).unwrap_err(), KeyError::Missing);
        assert_eq!(*m.1.lock().unwrap(), 0);
        let k = key_for(&m, 7, true).unwrap();
        let again = key_for(&m, 7, true).unwrap();
        assert_eq!(k.0, again.0);
        assert_eq!(*m.1.lock().unwrap(), 1, "made once");
        // Each journal its own key.
        let other = key_for(&m, 8, true).unwrap();
        assert_ne!(k.0, other.0);
        assert_eq!(format!("{k:?}"), "TagKey(redacted)");
    }

    #[test]
    fn a_tag_names_its_record_its_number_and_every_byte() {
        let k = TagKey::from_bytes([3; 32]);
        let t = k.tag("effects", 1, b"abc");
        assert!(k.verify("effects", 1, b"abc", &t));
        assert!(!k.verify("effects", 2, b"abc", &t));
        assert!(!k.verify("effect-undo", 1, b"abc", &t));
        assert!(!k.verify("effects", 1, b"abd", &t));
        assert!(!TagKey::from_bytes([4; 32]).verify("effects", 1, b"abc", &t));
        // The table's name and the record cannot be shifted into each other.
        assert_ne!(k.tag("effects", 1, b"xabc"), k.tag("effectsx", 1, b"abc"));
    }

    /// The system's own store, for real: a key is kept, read back, and removed again (this test
    /// leaves nothing behind). Windows: Credential Manager; macOS: the login Keychain; Linux: the
    /// person's own key folder.
    #[test]
    fn the_system_store_keeps_a_key() {
        let id = u64::from_le_bytes(*b"pctwtest") ^ u64::from(std::process::id());
        let made = key_for(&SystemStore, id, true).expect("the system's key store works here");
        let back = SystemStore.get(id).unwrap().unwrap();
        assert_eq!(made.0, back.0);
        forget(id);
        assert!(SystemStore.get(id).unwrap().is_none());
    }

    /// Removes a test key (tests only: PCTwin never removes a journal's key).
    fn forget(id: u64) {
        #[cfg(any(windows, target_os = "macos"))]
        SystemStore::entry(id).unwrap().delete_credential().unwrap();
        #[cfg(target_os = "linux")]
        std::fs::remove_file(SystemStore::folder().unwrap().join(entry_name(id))).unwrap();
    }
}
