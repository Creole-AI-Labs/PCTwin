//! A plaintext export's life (plan sections 7 and 10; acceptance case T25-022).
//!
//! Some apps' own export of saved passwords writes a readable file. PCTwin never makes one
//! itself: the person runs the app's own export, after being told what it will hold
//! ([`DISCLOSURE`]). PCTwin offers a protected place for it ([`ExportSlot::prepare`]: a new folder
//! only the person can open, inside PCTwin's own local folder, which is never a synced one), and
//! removes the file only when all of these hold ([`ExportSlot::cleanup`]):
//!
//! - PCTwin made the place (a file the person saved elsewhere is theirs: never removed);
//! - the import on the new laptop was verified;
//! - the file is still exactly what was imported (a file changed since is kept).
//!
//! Removal is never called secure erasure: a drive (an SSD especially) can keep traces the system
//! does not let any app erase, and the words say so.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::land::OwnerOnly;

/// What the person is told before the app's own export writes a readable file.
pub const DISCLOSURE: &str = "Your app will save your passwords in a file anyone who opens it can read. PCTwin keeps that file in a folder only you can open, uses it only to bring your passwords in on the new laptop, and removes it once they are in. It is never sent anywhere else.";

/// What removal says, honestly.
pub const REMOVED_WORDS: &str = "The password file PCTwin kept is removed. Some drives keep traces of removed files that no app can erase; your passwords are now in your app on the new laptop.";

/// What was in the export when it was imported: its size and BLAKE3 hash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sealed {
    pub len: u64,
    pub hash: [u8; 32],
}

/// The place one export is written to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportSlot {
    folder: Option<PathBuf>,
    file: PathBuf,
    ours: bool,
}

/// How cleaning up an export ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cleanup {
    /// PCTwin's own copy is removed.
    Removed,
    /// Kept: the import was not verified, so this may be the only copy.
    KeptImportNotVerified,
    /// Kept: the person saved it themselves, elsewhere; it is theirs.
    KeptNotOurs,
    /// Kept: it changed since it was imported.
    KeptChanged,
    /// Nothing is there any more.
    Gone,
}

static NEXT: AtomicU64 = AtomicU64::new(0);

fn read_sealed(path: &Path) -> std::io::Result<Sealed> {
    let mut f = std::fs::File::open(path)?;
    let mut h = blake3::Hasher::new();
    let mut buf = vec![0u8; 1 << 16];
    let mut len = 0u64;
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
        len += n as u64;
    }
    Ok(Sealed {
        len,
        hash: *h.finalize().as_bytes(),
    })
}

impl ExportSlot {
    /// A new owner-only folder inside `base` (PCTwin's own local folder) for one export file named
    /// `file_name` (a plain name the app's export suggests).
    pub fn prepare(
        base: &Path,
        file_name: &str,
        owner_only: &dyn OwnerOnly,
    ) -> std::io::Result<Self> {
        if file_name.is_empty()
            || file_name.contains(['/', '\\', ':', '\0'])
            || file_name == "."
            || file_name == ".."
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "a plain file name",
            ));
        }
        std::fs::create_dir_all(base)?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let n = NEXT.fetch_add(1, Ordering::SeqCst);
        let id = blake3::hash(format!("{stamp}-{}-{n}", std::process::id()).as_bytes());
        let folder = base.join(format!("export-{}", &id.to_hex()[..16]));
        owner_only.private_folder(&folder)?;
        Ok(Self {
            file: folder.join(file_name),
            folder: Some(folder),
            ours: true,
        })
    }

    /// An export the person saved themselves, at `path`: PCTwin reads it, and never removes it.
    pub fn adopt(path: PathBuf) -> Self {
        Self {
            folder: None,
            file: path,
            ours: false,
        }
    }

    /// Where the export is written.
    pub fn file(&self) -> &Path {
        &self.file
    }

    /// Whether PCTwin made this place.
    pub fn ours(&self) -> bool {
        self.ours
    }

    /// What the export holds now (recorded when it is imported).
    pub fn seal(&self) -> std::io::Result<Sealed> {
        read_sealed(&self.file)
    }

    /// Removes PCTwin's own export after a verified import of exactly `imported`; keeps it in every
    /// other case.
    pub fn cleanup(self, imported: Option<Sealed>) -> std::io::Result<Cleanup> {
        if !self.ours {
            return Ok(Cleanup::KeptNotOurs);
        }
        let Some(imported) = imported else {
            return Ok(Cleanup::KeptImportNotVerified);
        };
        let now = match read_sealed(&self.file) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Cleanup::Gone),
            Err(e) => return Err(e),
        };
        if now != imported {
            return Ok(Cleanup::KeptChanged);
        }
        std::fs::remove_file(&self.file)?;
        if let Some(folder) = &self.folder {
            // Only an empty folder goes (anything else in it is not PCTwin's to remove).
            let _ = std::fs::remove_dir(folder);
        }
        Ok(Cleanup::Removed)
    }
}
