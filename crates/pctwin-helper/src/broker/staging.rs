//! Packages waiting to be installed, kept in the helper's own folder, which only administrators
//! and the system can open.
//!
//! A package is only ever installed from the helper's own copy, never from where the app or a
//! download put it, and is checked on the very file the installer is given:
//!
//! 0. [`Staging::new`] empties the folder first: a package left by a helper that crashed belongs
//!    to no session any more (security review finding 7: a stale file under a counted name used to
//!    jam staging for good).
//! 1. [`Staging::admit`] copies the bytes into a new file of the helper's own, under a random
//!    name the helper chose (never one made before, so nothing left behind can be in its way),
//!    hashing as it writes; a copy whose SHA-256 is not the approved one is removed.
//! 2. [`Staging::open_verified`], at the moment of use, opens that file without following a link,
//!    on Windows so that nobody may write, rename or delete it while it is open (and fails if
//!    anyone already holds it open for writing), checks it is a plain file, and hashes it again
//!    through that same open handle. The [`VerifiedArtifact`] keeps the file open until the
//!    installer has finished, so what was checked is what is installed.

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// The largest package the helper keeps.
pub const MAX_PACKAGE: u64 = 4 * 1024 * 1024 * 1024;

/// Why a package could not be kept or used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageError {
    /// No package with that handle.
    Unknown,
    /// Its bytes are not the approved ones (at admission, or changed since).
    Changed,
    /// It is gone, or is no longer a plain file (a link was put in its place).
    NotPlain,
    /// Someone has it open for writing, so it was not used.
    InUse,
    /// Larger than [`MAX_PACKAGE`].
    TooLarge,
    /// It could not be read or written.
    Io,
}

/// The helper's own packages, by handle.
pub struct Staging {
    root: PathBuf,
    next: u64,
    kept: BTreeMap<u64, PathBuf>,
    limit: u64,
}

impl Staging {
    /// Packages kept in `root`, a folder of the helper's own that only administrators and the
    /// system can open (`pctwin_helper_os::admin_only_folder` on Windows). Everything already in
    /// it is removed first (only the one helper of a move uses it, and none is running before it
    /// starts); a link there is removed itself, never followed.
    pub fn new(root: PathBuf) -> Self {
        if let Ok(list) = std::fs::read_dir(&root) {
            for entry in list.flatten() {
                let _ = std::fs::remove_file(entry.path());
            }
        }
        Self {
            root,
            next: 1,
            kept: BTreeMap::new(),
            limit: MAX_PACKAGE,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_limit(root: PathBuf, limit: u64) -> Self {
        Self {
            limit,
            ..Self::new(root)
        }
    }

    /// Copies a package from `from` into a new file named by the helper, and keeps it only if its
    /// SHA-256 is `expected`. Its handle is returned.
    pub fn admit(&mut self, from: &mut impl Read, expected: [u8; 32]) -> Result<u64, StageError> {
        let handle = self.next;
        let mut bits = [0u8; 16];
        getrandom::fill(&mut bits).map_err(|_| StageError::Io)?;
        let name: String = bits.iter().map(|b| format!("{b:02x}")).collect();
        let path = self.root.join(format!("p-{name}.pkg"));
        let mut out = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|_| StageError::Io)?;
        let copied = copy_hashing(from, &mut out, self.limit);
        drop(out);
        match copied {
            Ok(digest) if digest == expected => {
                self.next += 1;
                self.kept.insert(handle, path);
                Ok(handle)
            }
            other => {
                let _ = std::fs::remove_file(&path);
                Err(other.err().unwrap_or(StageError::Changed))
            }
        }
    }

    /// The package `handle`, opened and checked again as described above.
    pub fn open_verified(
        &self,
        handle: u64,
        expected: [u8; 32],
    ) -> Result<VerifiedArtifact, StageError> {
        let path = self.kept.get(&handle).ok_or(StageError::Unknown)?;
        let mut file = open_held(path)?;
        let meta = file.metadata().map_err(|_| StageError::NotPlain)?;
        if !meta.is_file() || is_link(&meta) {
            return Err(StageError::NotPlain);
        }
        let digest = hash(&mut file)?;
        if digest != expected {
            return Err(StageError::Changed);
        }
        Ok(VerifiedArtifact {
            _held: file,
            path: path.clone(),
            sha256: digest,
        })
    }

    /// Removes every package (when the session ends).
    pub fn clear(&mut self) {
        for path in std::mem::take(&mut self.kept).into_values() {
            let _ = std::fs::remove_file(path);
        }
    }

    #[cfg(test)]
    pub(crate) fn path_of(&self, handle: u64) -> Option<&Path> {
        self.kept.get(&handle).map(PathBuf::as_path)
    }
}

/// A package checked at the moment of use and held open, unchanged, until dropped.
#[derive(Debug)]
pub struct VerifiedArtifact {
    _held: File,
    path: PathBuf,
    sha256: [u8; 32],
}

impl VerifiedArtifact {
    /// Where it is, in the helper's own folder: for the installer, while this is held.
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn sha256(&self) -> [u8; 32] {
        self.sha256
    }
}

fn copy_hashing(from: &mut impl Read, to: &mut File, limit: u64) -> Result<[u8; 32], StageError> {
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    let mut total = 0u64;
    loop {
        let n = from.read(&mut buf).map_err(|_| StageError::Io)?;
        if n == 0 {
            break;
        }
        total += n as u64;
        if total > limit {
            return Err(StageError::TooLarge);
        }
        h.update(&buf[..n]);
        to.write_all(&buf[..n]).map_err(|_| StageError::Io)?;
    }
    to.sync_all().map_err(|_| StageError::Io)?;
    Ok(h.finalize().into())
}

fn hash(file: &mut File) -> Result<[u8; 32], StageError> {
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf).map_err(|_| StageError::Io)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().into())
}

/// Windows: opens without following a link (the link itself is opened, and then refused), and
/// lets others only read while it is open: no writing, renaming or deleting. If someone already
/// has it open for writing, the open fails.
#[cfg(windows)]
fn open_held(path: &Path) -> Result<File, StageError> {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_SHARE_READ: u32 = 0x1;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    const ERROR_SHARING_VIOLATION: i32 = 32;
    OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
        .map_err(|e| match e.raw_os_error() {
            Some(ERROR_SHARING_VIOLATION) => StageError::InUse,
            _ if e.kind() == std::io::ErrorKind::NotFound => StageError::NotPlain,
            _ => StageError::Io,
        })
}

/// Mac and Linux: opens without following a link. Only the system can write in the helper's
/// folder, so nobody else can change the file while it is used.
#[cfg(unix)]
fn open_held(path: &Path) -> Result<File, StageError> {
    use std::os::unix::fs::OpenOptionsExt;
    OpenOptions::new()
        .read(true)
        .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
        .open(path)
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => StageError::NotPlain,
            // ELOOP: the last step is a link.
            _ if e.raw_os_error() == Some(rustix::io::Errno::LOOP.raw_os_error()) => {
                StageError::NotPlain
            }
            _ => StageError::Io,
        })
}

#[cfg(windows)]
fn is_link(meta: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(unix)]
fn is_link(meta: &std::fs::Metadata) -> bool {
    meta.file_type().is_symlink()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sha(bytes: &[u8]) -> [u8; 32] {
        Sha256::digest(bytes).into()
    }

    #[test]
    fn a_package_is_kept_only_with_its_approved_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Staging::new(dir.path().to_path_buf());
        let bytes = b"a package".to_vec();
        let h = s.admit(&mut bytes.as_slice(), sha(&bytes)).unwrap();
        assert_eq!(h, 1);
        let v = s.open_verified(h, sha(&bytes)).unwrap();
        assert_eq!(v.sha256(), sha(&bytes));
        assert!(v.path().starts_with(dir.path()));
        assert_eq!(std::fs::read(v.path()).unwrap(), bytes);
        drop(v);
        // Wrong bytes are not kept, and leave nothing behind.
        assert_eq!(
            s.admit(&mut &b"other"[..], sha(&bytes)).unwrap_err(),
            StageError::Changed
        );
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
        let h2 = s.admit(&mut &b"two"[..], sha(b"two")).unwrap();
        assert_eq!(h2, 2);
        assert_eq!(
            s.open_verified(9, sha(b"two")).unwrap_err(),
            StageError::Unknown
        );
        assert_eq!(
            s.open_verified(h2, sha(b"three")).unwrap_err(),
            StageError::Changed
        );
        s.clear();
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
        assert_eq!(
            s.open_verified(h, sha(&bytes)).unwrap_err(),
            StageError::Unknown
        );
    }

    /// T22-016: the package changed after it was checked and before the installer started is
    /// refused, because it is checked again, on the open file, at the moment of use.
    #[test]
    fn a_package_swapped_after_checking_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Staging::new(dir.path().to_path_buf());
        let bytes = b"the real package".to_vec();
        let h = s.admit(&mut bytes.as_slice(), sha(&bytes)).unwrap();
        // Checked once (as when the request was validated) ...
        drop(s.open_verified(h, sha(&bytes)).unwrap());
        // ... then swapped before use.
        std::fs::write(s.path_of(h).unwrap(), b"a rogue package!").unwrap();
        assert_eq!(
            s.open_verified(h, sha(&bytes)).unwrap_err(),
            StageError::Changed
        );
        // Gone altogether.
        std::fs::remove_file(s.path_of(h).unwrap()).unwrap();
        assert_eq!(
            s.open_verified(h, sha(&bytes)).unwrap_err(),
            StageError::NotPlain
        );
    }

    /// T22-016 on Windows, for real: while the checked package is held, nobody can write, rename
    /// or delete it; and a package someone holds open for writing is not used.
    #[cfg(windows)]
    #[test]
    fn while_held_nobody_can_change_it() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Staging::new(dir.path().to_path_buf());
        let bytes = b"held package".to_vec();
        let h = s.admit(&mut bytes.as_slice(), sha(&bytes)).unwrap();
        let path = s.path_of(h).unwrap().to_path_buf();
        let held = s.open_verified(h, sha(&bytes)).unwrap();
        assert!(OpenOptions::new().write(true).open(&path).is_err());
        assert!(std::fs::rename(&path, dir.path().join("moved")).is_err());
        assert!(std::fs::remove_file(&path).is_err());
        // Reading is still allowed (the installer reads it).
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        drop(held);
        let writer = OpenOptions::new().write(true).open(&path).unwrap();
        assert_eq!(
            s.open_verified(h, sha(&bytes)).unwrap_err(),
            StageError::InUse
        );
        drop(writer);
        assert!(s.open_verified(h, sha(&bytes)).is_ok());
    }

    /// T22-017: a link put in place of the package is never followed.
    #[test]
    fn a_link_in_place_of_the_package_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Staging::new(dir.path().to_path_buf());
        let bytes = b"pkg".to_vec();
        let h = s.admit(&mut bytes.as_slice(), sha(&bytes)).unwrap();
        let path = s.path_of(h).unwrap().to_path_buf();
        // The link points at a file with the very same bytes: only the link itself is wrong.
        let elsewhere = dir.path().join("elsewhere");
        std::fs::write(&elsewhere, &bytes).unwrap();
        std::fs::remove_file(&path).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&elsewhere, &path).unwrap();
        #[cfg(windows)]
        if std::os::windows::fs::symlink_file(&elsewhere, &path).is_err() {
            // Making a link needs Developer Mode or an administrator on Windows: proven on the
            // Unix runners and wherever links can be made.
            eprintln!("links cannot be made here; not proven on this run");
            return;
        }
        assert_eq!(
            s.open_verified(h, sha(&bytes)).unwrap_err(),
            StageError::NotPlain
        );
    }

    /// Review finding 7: a package a crashed helper left behind never jams staging, and is gone
    /// once the next helper starts.
    #[test]
    fn a_stale_package_from_a_crash_never_jams_staging() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("p1.pkg"), b"left by a crash").unwrap();
        let mut s = Staging::new(dir.path().to_path_buf());
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
        // Even a file appearing later under a name an earlier scheme used is never in the way.
        std::fs::write(dir.path().join("p1.pkg"), b"left by a crash").unwrap();
        let bytes = b"the package".to_vec();
        let h = s.admit(&mut bytes.as_slice(), sha(&bytes)).unwrap();
        // A file under the very name the next package would get (if names were counted).
        let first = s.path_of(h).unwrap().to_path_buf();
        let h2 = s.admit(&mut bytes.as_slice(), sha(&bytes)).unwrap();
        assert_ne!(s.path_of(h2).unwrap(), first, "every package its own name");
        assert_eq!(std::fs::read(s.path_of(h2).unwrap()).unwrap(), bytes);
    }

    #[test]
    fn a_package_too_large_is_not_kept() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Staging::with_limit(dir.path().to_path_buf(), 10);
        let ten = [7u8; 10];
        assert!(s.admit(&mut &ten[..], sha(&ten)).is_ok());
        let eleven = [7u8; 11];
        assert_eq!(
            s.admit(&mut &eleven[..], sha(&eleven)).unwrap_err(),
            StageError::TooLarge
        );
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
        assert_eq!(Staging::new(dir.path().to_path_buf()).limit, MAX_PACKAGE);
    }
}
