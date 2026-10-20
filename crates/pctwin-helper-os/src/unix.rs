//! macOS and Linux: the helper's private channel, a Unix socket in a folder only the mover's
//! account can enter, and who is calling. No unsafe code here.

use crate::{Caller, Error};
use rustix::fs::{Mode, OFlags};
use std::fs::DirBuilder;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

/// Folder mode: the owner may enter, list and write; no one else may do anything.
pub(crate) const PRIVATE_MODE: u32 = 0o700;
/// Socket mode: only the owner may connect.
pub(crate) const SOCKET_MODE: u32 = 0o600;

/// A folder only one account (and root) can enter: a real folder, not a link, owned by that
/// account, mode exactly 0700, inside a parent no other account can rename it out of.
#[derive(Debug)]
pub struct PrivateDir {
    path: PathBuf,
    owner: u32,
}

impl PrivateDir {
    /// Makes the folder at `path` (which must not exist yet) for `owner`, then checks it. When
    /// run as root for another account, the folder is handed to that account.
    pub fn create(path: &Path, owner: u32) -> Result<PrivateDir, Error> {
        DirBuilder::new().mode(PRIVATE_MODE).create(path)?;
        // The process's umask can only remove bits; set the mode exactly.
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(PRIVATE_MODE))?;
        if rustix::process::geteuid().as_raw() != owner {
            std::os::unix::fs::lchown(path, Some(owner), None)?;
        }
        PrivateDir::open(path, owner)
    }

    /// Checks an existing folder is private to `owner`.
    pub fn open(path: &Path, owner: u32) -> Result<PrivateDir, Error> {
        let dir = rustix::fs::open(
            path,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|e| Error::NotPrivate(format!("cannot open it as a folder (not a link): {e}")))?;
        let st = rustix::fs::fstat(&dir).map_err(io)?;
        check_private(st.st_uid, bits(st.st_mode), owner)?;
        let parent = path
            .parent()
            .ok_or_else(|| Error::NotPrivate("it has no parent folder".into()))?;
        let pst = rustix::fs::stat(parent).map_err(io)?;
        check_parent(pst.st_uid, bits(pst.st_mode), owner)?;
        Ok(PrivateDir {
            path: path.to_path_buf(),
            owner,
        })
    }

    /// Where it is.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The one account that may enter it.
    pub fn owner(&self) -> u32 {
        self.owner
    }
}

/// A mode as 32 bits (16 bits on macOS, 32 on Linux).
fn bits(mode: impl Into<u32>) -> u32 {
    mode.into()
}

fn io(e: rustix::io::Errno) -> Error {
    Error::Io(e.into())
}

/// Owned by `owner`, and no permission for group or others, nor set-ID or sticky bits.
pub(crate) fn check_private(uid: u32, mode: u32, owner: u32) -> Result<(), Error> {
    if uid != owner {
        return Err(Error::NotPrivate(format!(
            "owned by account {uid}, not {owner}"
        )));
    }
    if mode & 0o7777 != PRIVATE_MODE {
        return Err(Error::NotPrivate(format!(
            "mode {:o}, not 700",
            mode & 0o7777
        )));
    }
    Ok(())
}

/// The parent belongs to root or the owner, and no one else can write to it unless it is sticky
/// (like /tmp, where only an entry's owner may rename or remove it).
pub(crate) fn check_parent(uid: u32, mode: u32, owner: u32) -> Result<(), Error> {
    if uid != 0 && uid != owner {
        return Err(Error::NotPrivate(format!(
            "its parent folder belongs to account {uid}"
        )));
    }
    if mode & 0o022 != 0 && mode & 0o1000 == 0 {
        return Err(Error::NotPrivate(
            "others may rename it out of its parent folder".into(),
        ));
    }
    Ok(())
}

/// A socket name: `h-` and 16 random hexadecimal characters, short enough for the Unix limit on
/// socket paths (104 bytes on macOS) inside a short folder.
pub fn socket_name() -> Result<String, Error> {
    Ok(format!("h-{}.sock", &crate::random_token()?[..16]))
}

/// The helper's end of the private channel.
#[derive(Debug)]
pub struct SocketServer {
    listener: UnixListener,
    path: PathBuf,
}

impl SocketServer {
    /// Makes the socket `name` in `dir`, owned by the folder's owner, mode 0600.
    /// [`Error::NameTaken`] if something of that name is already there: never reused.
    pub fn bind(dir: &PrivateDir, name: &str) -> Result<SocketServer, Error> {
        if name.is_empty()
            || name.len() > 32
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.')
            || name.starts_with('.')
        {
            return Err(Error::BadInput("not a socket name"));
        }
        // Checked again: the folder must still be private when the socket appears in it.
        let dir = PrivateDir::open(&dir.path, dir.owner)?;
        let path = dir.path.join(name);
        let listener = match UnixListener::bind(&path) {
            Ok(l) => l,
            Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => return Err(Error::NameTaken),
            Err(e) => return Err(e.into()),
        };
        let server = SocketServer { listener, path };
        std::fs::set_permissions(&server.path, std::fs::Permissions::from_mode(SOCKET_MODE))?;
        if rustix::process::geteuid().as_raw() != dir.owner {
            std::os::unix::fs::lchown(&server.path, Some(dir.owner), None)?;
        }
        Ok(server)
    }

    /// Waits for a connection and says who made it.
    pub fn accept(&self) -> Result<(UnixStream, Caller), Error> {
        let (stream, _) = self.listener.accept()?;
        let caller = caller_of(&stream)?;
        Ok((stream, caller))
    }

    /// Where the socket is.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for SocketServer {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Who is on the other end of a connected Unix socket: the process ID and account the system
/// recorded when it connected, and the program that process runs. Works from either end (the
/// app checks the helper is root and the process it started).
///
/// Not closed here (for the fresh-context review): the program path is read after the
/// connection, by process ID; a client that ends at once could have its ID reused before the
/// path is read. The account is the one recorded at connection time and cannot be swapped.
pub fn caller_of(stream: &UnixStream) -> Result<Caller, Error> {
    #[cfg(target_os = "linux")]
    {
        crate::linux::peer(stream)
    }
    #[cfg(target_os = "macos")]
    {
        crate::mac::peer(stream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_owner_only_0700_is_private() {
        assert!(check_private(1000, 0o40700, 1000).is_ok());
        assert!(check_private(1001, 0o40700, 1000).is_err());
        assert!(check_private(0, 0o40700, 1000).is_err());
        for mode in [
            0o710, 0o701, 0o750, 0o755, 0o777, 0o600, 0o500, 0o1700, 0o2700, 0o4700,
        ] {
            assert!(
                check_private(1000, 0o40000 | mode, 1000).is_err(),
                "{mode:o}"
            );
        }
    }

    #[test]
    fn parent_must_not_let_others_rename_it() {
        assert!(check_parent(0, 0o755, 1000).is_ok());
        assert!(check_parent(1000, 0o700, 1000).is_ok());
        assert!(check_parent(0, 0o1777, 1000).is_ok());
        assert!(check_parent(1001, 0o755, 1000).is_err());
        assert!(check_parent(0, 0o777, 1000).is_err());
        assert!(check_parent(0, 0o775, 1000).is_err());
        assert!(check_parent(1000, 0o757, 1000).is_err());
    }

    #[test]
    fn socket_names_fit() {
        let n = socket_name().unwrap();
        assert_eq!(n.len(), 2 + 16 + 5);
    }
}
