//! macOS and Linux: the helper's private channel, a Unix socket in a folder only the mover's
//! account can enter, and who is calling. No unsafe code here.
//!
//! Every step after the parent folder is checked works through open handles (`mkdirat`,
//! `openat` with `O_NOFOLLOW`, `fchmod`, `fchown`), never by path, so a link or folder swapped
//! in by name between a check and a change is never followed.

use crate::{Caller, Error};
use rustix::fs::{AtFlags, FileType, Mode, OFlags};
use std::os::fd::OwnedFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Component, Path, PathBuf};

/// Folder mode: the owner may enter, list and write; no one else may do anything.
pub(crate) const PRIVATE_MODE: u32 = 0o700;
/// Socket mode: only the owner may connect.
pub(crate) const SOCKET_MODE: u32 = 0o600;

/// A folder only one account (and root) can enter: a real folder, not a link, owned by that
/// account, mode exactly 0700, inside a parent no other account can rename it out of. It keeps
/// the folder open; everything made inside it is made through that handle.
#[derive(Debug)]
pub struct PrivateDir {
    path: PathBuf,
    owner: u32,
    fd: OwnedFd,
}

fn dir_flags() -> OFlags {
    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC
}

fn mode(bits: u32) -> Mode {
    Mode::from_bits_truncate(bits as _)
}

/// Splits `path` into its parent, opened (not through a link) and checked first, and its last
/// name.
fn checked_parent(path: &Path, owner: u32) -> Result<(OwnedFd, PathBuf), Error> {
    if path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(Error::BadInput("the folder's path may not hold \"..\""));
    }
    let name = match path.components().next_back() {
        Some(Component::Normal(n)) => PathBuf::from(n),
        _ => return Err(Error::BadInput("the folder must end in a plain name")),
    };
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or(Error::BadInput("the folder must be a full path"))?;
    let pfd = rustix::fs::open(parent, dir_flags(), Mode::empty()).map_err(|e| {
        Error::NotPrivate(format!(
            "cannot open its parent as a folder (not a link): {e}"
        ))
    })?;
    let pst = rustix::fs::fstat(&pfd).map_err(io)?;
    check_parent(pst.st_uid, bits(pst.st_mode), owner)?;
    Ok((pfd, name))
}

impl PrivateDir {
    /// Makes the folder at `path` (which must not exist yet) for `owner`. The parent is checked
    /// before anything is made; the folder is made through the parent's handle, opened without
    /// following a link, set to mode 0700 and handed to `owner` through its own handle, then
    /// checked again.
    pub fn create(path: &Path, owner: u32) -> Result<PrivateDir, Error> {
        let (pfd, name) = checked_parent(path, owner)?;
        rustix::fs::mkdirat(&pfd, &name, mode(PRIVATE_MODE)).map_err(io)?;
        let fd = rustix::fs::openat(&pfd, &name, dir_flags(), Mode::empty()).map_err(|e| {
            Error::NotPrivate(format!(
                "the new folder was swapped before it was opened: {e}"
            ))
        })?;
        // The process's umask can only remove bits; set the mode exactly, on the handle.
        rustix::fs::fchmod(&fd, mode(PRIVATE_MODE)).map_err(io)?;
        if rustix::process::geteuid().as_raw() != owner {
            rustix::fs::fchown(&fd, Some(rustix::fs::Uid::from_raw_unchecked(owner)), None)
                .map_err(io)?;
        }
        let st = rustix::fs::fstat(&fd).map_err(io)?;
        check_private(st.st_uid, bits(st.st_mode), owner)?;
        Ok(PrivateDir {
            path: path.to_path_buf(),
            owner,
            fd,
        })
    }

    /// Checks an existing folder is private to `owner`: its parent first, then the folder
    /// itself, opened through the parent's handle without following a link.
    pub fn open(path: &Path, owner: u32) -> Result<PrivateDir, Error> {
        let (pfd, name) = checked_parent(path, owner)?;
        let fd = rustix::fs::openat(&pfd, &name, dir_flags(), Mode::empty()).map_err(|e| {
            Error::NotPrivate(format!("cannot open it as a folder (not a link): {e}"))
        })?;
        let st = rustix::fs::fstat(&fd).map_err(io)?;
        check_private(st.st_uid, bits(st.st_mode), owner)?;
        Ok(PrivateDir {
            path: path.to_path_buf(),
            owner,
            fd,
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

    /// Its path still names the very folder this handle holds (not one swapped in), and that
    /// folder is still private.
    fn path_is_still_it(&self) -> Result<(), Error> {
        let held = rustix::fs::fstat(&self.fd).map_err(io)?;
        let named = PrivateDir::open(&self.path, self.owner)?;
        let now = rustix::fs::fstat(&named.fd).map_err(io)?;
        if (held.st_dev, held.st_ino) != (now.st_dev, now.st_ino) {
            return Err(Error::NotPrivate(
                "its path now names a different folder".into(),
            ));
        }
        check_private(held.st_uid, bits(held.st_mode), self.owner)
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
    ///
    /// Binding takes a path, so the folder's path is checked to name the held folder before and
    /// after; the entry is then confirmed to be this new socket, and its mode and owner are set
    /// relative to the folder's handle, never through the folder's path.
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
        dir.path_is_still_it()?;
        let path = dir.path.join(name);
        let listener = match UnixListener::bind(&path) {
            Ok(l) => l,
            Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => return Err(Error::NameTaken),
            Err(e) => return Err(e.into()),
        };
        let server = SocketServer { listener, path };
        dir.path_is_still_it()?;
        let st = rustix::fs::statat(&dir.fd, name, AtFlags::SYMLINK_NOFOLLOW).map_err(io)?;
        if FileType::from_raw_mode(st.st_mode) != FileType::Socket
            || st.st_uid != rustix::process::geteuid().as_raw()
        {
            return Err(Error::NotPrivate("the socket was swapped".into()));
        }
        rustix::fs::chmodat(&dir.fd, name, mode(SOCKET_MODE), AtFlags::empty()).map_err(io)?;
        if rustix::process::geteuid().as_raw() != dir.owner {
            rustix::fs::chownat(
                &dir.fd,
                name,
                Some(rustix::fs::Uid::from_raw_unchecked(dir.owner)),
                None,
                AtFlags::SYMLINK_NOFOLLOW,
            )
            .map_err(io)?;
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

/// Who is on the other end of a connected Unix socket. Works from either end (the app checks
/// the helper is root and the process it started).
///
/// Only [`Caller::account`] is authoritative: the kernel records it when the connection is made
/// and it cannot change afterwards. The process ID is the one recorded then, and
/// [`Caller::program_hint`] is read afterwards by that ID (`/proc/<pid>/exe`, `proc_pidpath`),
/// so a process that starts another program, or ends and has its ID reused, can change it: it
/// is for the record and the report, never for deciding what is allowed.
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
