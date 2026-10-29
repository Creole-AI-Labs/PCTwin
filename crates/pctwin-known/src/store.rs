//! Where each remembered connection's secrets live (Security Design Part T and Part G): its
//! private key and the pair's secret, never in PCTwin's own files.
//!
//! - **Windows**: the person's Credential Manager (a local, not roaming, credential), which
//!   Windows encrypts with the person's own data-protection key (DPAPI).
//! - **macOS**: the person's login Keychain.
//! - **Linux**: a file only the person can read (`~/.config/pctwin/known-keys/`, folder `0700`,
//!   each file `0600`, both checked to be the person's own). This is the weaker fallback Part T
//!   allows and the app says so ([`SystemStore::is_fallback`]): any program running as the person
//!   can read it. The Secret Service is not used yet.
//!
//! Each system store is reached only while the person's session is open: Credential Manager and
//! the login Keychain are the signed-in person's, and the app reads a key only for a session it
//! is about to run ([`crate::Book`]), never to keep it in memory.

use zeroize::Zeroizing;

/// The store could not be reached, or a secret in it is not what PCTwin keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum StoreError {
    #[error("the system's key store could not be reached")]
    Unavailable,
    #[error("a key in the system's key store is not in the expected form")]
    Damaged,
}

/// A place secrets are kept by name.
pub trait SecretStore: Send + Sync {
    fn get(&self, name: &str) -> Result<Option<Zeroizing<Vec<u8>>>, StoreError>;
    fn put(&self, name: &str, secret: &[u8]) -> Result<(), StoreError>;
    /// Removes `name`; one that is not there is already removed.
    fn delete(&self, name: &str) -> Result<(), StoreError>;
}

/// Secrets in this process's memory only: for tests.
#[derive(Default)]
pub struct MemoryStore(std::sync::Mutex<std::collections::BTreeMap<String, Zeroizing<Vec<u8>>>>);

impl MemoryStore {
    /// The names held, for tests that check Forget removed everything.
    pub fn names(&self) -> Vec<String> {
        self.0
            .lock()
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default()
    }
}

impl SecretStore for MemoryStore {
    fn get(&self, name: &str) -> Result<Option<Zeroizing<Vec<u8>>>, StoreError> {
        let m = self.0.lock().map_err(|_| StoreError::Unavailable)?;
        Ok(m.get(name).cloned())
    }
    fn put(&self, name: &str, secret: &[u8]) -> Result<(), StoreError> {
        let mut m = self.0.lock().map_err(|_| StoreError::Unavailable)?;
        m.insert(name.to_string(), Zeroizing::new(secret.to_vec()));
        Ok(())
    }
    fn delete(&self, name: &str) -> Result<(), StoreError> {
        let mut m = self.0.lock().map_err(|_| StoreError::Unavailable)?;
        m.remove(name);
        Ok(())
    }
}

/// The system's own key store for this person.
pub struct SystemStore;

impl SystemStore {
    /// Whether this system keeps the keys in the weaker file fallback (Linux), which the app says.
    pub fn is_fallback() -> bool {
        cfg!(not(any(windows, target_os = "macos")))
    }
}

#[cfg(any(windows, target_os = "macos"))]
const SERVICE: &str = "PCTwin remembered connection";

#[cfg(any(windows, target_os = "macos"))]
impl SystemStore {
    fn entry(name: &str) -> Result<keyring_core::Entry, StoreError> {
        #[cfg(windows)]
        let store =
            windows_native_keyring_store::Store::new().map_err(|_| StoreError::Unavailable)?;
        #[cfg(target_os = "macos")]
        let store = apple_native_keyring_store::keychain::Store::new()
            .map_err(|_| StoreError::Unavailable)?;
        #[cfg(windows)]
        let modifiers = std::collections::HashMap::from([("persistence", "Local")]);
        #[cfg(target_os = "macos")]
        let modifiers = std::collections::HashMap::new();
        keyring_core::api::CredentialStoreApi::build(
            store.as_ref(),
            SERVICE,
            name,
            Some(&modifiers),
        )
        .map_err(|_| StoreError::Unavailable)
    }
}

#[cfg(any(windows, target_os = "macos"))]
impl SecretStore for SystemStore {
    fn get(&self, name: &str) -> Result<Option<Zeroizing<Vec<u8>>>, StoreError> {
        match Self::entry(name)?.get_secret() {
            Ok(bytes) => Ok(Some(Zeroizing::new(bytes))),
            Err(keyring_core::Error::NoEntry) => Ok(None),
            Err(_) => Err(StoreError::Unavailable),
        }
    }
    fn put(&self, name: &str, secret: &[u8]) -> Result<(), StoreError> {
        Self::entry(name)?
            .set_secret(secret)
            .map_err(|_| StoreError::Unavailable)
    }
    fn delete(&self, name: &str) -> Result<(), StoreError> {
        match Self::entry(name)?.delete_credential() {
            Ok(()) | Err(keyring_core::Error::NoEntry) => Ok(()),
            Err(_) => Err(StoreError::Unavailable),
        }
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
impl SystemStore {
    /// `$XDG_CONFIG_HOME/pctwin/known-keys` (or `~/.config/...`), made `0700` if missing, and
    /// checked to be the person's own folder that nobody else can open.
    fn folder() -> Result<std::path::PathBuf, StoreError> {
        use std::os::unix::fs::{DirBuilderExt, MetadataExt};
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .filter(|v| !v.is_empty())
            .map(std::path::PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| std::path::Path::new(&h).join(".config")))
            .ok_or(StoreError::Unavailable)?;
        let dir = base.join("pctwin").join("known-keys");
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&dir)
            .map_err(|_| StoreError::Unavailable)?;
        let meta = std::fs::symlink_metadata(&dir).map_err(|_| StoreError::Unavailable)?;
        if !meta.is_dir()
            || meta.uid() != rustix::process::geteuid().as_raw()
            || meta.mode() & 0o077 != 0
        {
            return Err(StoreError::Unavailable);
        }
        Ok(dir)
    }

    /// Only names PCTwin makes: letters, digits and dashes.
    fn path(name: &str) -> Result<std::path::PathBuf, StoreError> {
        if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
            return Err(StoreError::Damaged);
        }
        Ok(Self::folder()?.join(name))
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
impl SecretStore for SystemStore {
    fn get(&self, name: &str) -> Result<Option<Zeroizing<Vec<u8>>>, StoreError> {
        use std::io::Read;
        use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
        let path = Self::path(name)?;
        let mut file = match std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
            .open(&path)
        {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(StoreError::Unavailable),
        };
        let meta = file.metadata().map_err(|_| StoreError::Unavailable)?;
        if !meta.is_file()
            || meta.uid() != rustix::process::geteuid().as_raw()
            || meta.mode() & 0o077 != 0
        {
            return Err(StoreError::Unavailable);
        }
        let mut bytes = Zeroizing::new(Vec::new());
        file.read_to_end(&mut bytes)
            .map_err(|_| StoreError::Unavailable)?;
        Ok(Some(bytes))
    }

    /// Written to a new file beside it (`0600`, made fresh), flushed, then moved over it.
    fn put(&self, name: &str, secret: &[u8]) -> Result<(), StoreError> {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let path = Self::path(name)?;
        let part = path.with_extension("part");
        let _ = std::fs::remove_file(&part);
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
            .open(&part)
            .map_err(|_| StoreError::Unavailable)?;
        file.write_all(secret)
            .and_then(|()| file.sync_all())
            .map_err(|_| StoreError::Unavailable)?;
        std::fs::rename(&part, &path).map_err(|_| StoreError::Unavailable)
    }

    fn delete(&self, name: &str) -> Result<(), StoreError> {
        match std::fs::remove_file(Self::path(name)?) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(StoreError::Unavailable),
        }
    }
}
