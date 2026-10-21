//! macOS and Linux: the helper's channel, a Unix socket in a folder only the helper's own
//! account can write, and who is calling. No unsafe code here.
//!
//! **Who owns what.** The server (the helper, running as root) owns the socket folder and is
//! the only account that can write in it: mode 0711, so other accounts may pass through to an
//! entry they know the name of, but may not list, add, remove or rename anything. Only the
//! socket entry inside is handed to the client's account (the mover), mode 0600, so only the
//! mover (and root) can connect. Because no less-privileged account can write in the folder,
//! nothing in it can be swapped under the helper: the helper never acts by name in anything
//! another account can write. The folder's parent must be one only root or the helper's own
//! account can write (or a sticky folder such as /tmp, where only the folder's owner may rename
//! or remove it), so the folder's path cannot be redirected either.
//!
//! The folder is made through the parent's handle (`mkdirat`), opened without following a link
//! (`openat` with `O_NOFOLLOW`) and set up through its own handle (`fchmod`); every later step
//! is relative to that held handle. Run without root (tests, or a non-root server), the same
//! rules hold with the server's own account in root's place.

use crate::{Account, Caller, Error};
use rustix::fs::{AtFlags, FileType, Mode, OFlags};
use std::os::fd::OwnedFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Component, Path, PathBuf};

/// Folder mode: the owner (the server) may do anything; others may only pass through.
pub(crate) const FOLDER_MODE: u32 = 0o711;
/// Socket mode: only the socket's owner (the client's account) may connect.
pub(crate) const SOCKET_MODE: u32 = 0o600;

/// The helper's socket folder: a real folder, not a link, owned by the account this process
/// runs as (root, for the helper), mode exactly 0711, inside a parent no other account can
/// rename it out of. It stays open; everything made inside it is made relative to that handle.
#[derive(Debug)]
pub struct ChannelDir {
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

fn euid() -> u32 {
    rustix::process::geteuid().as_raw()
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

impl ChannelDir {
    /// Makes the folder at `path` (which must not exist yet), owned by this process's account.
    /// The parent is checked before anything is made; the folder is made through the parent's
    /// handle, opened without following a link, set to mode 0711 through its own handle, then
    /// checked again.
    pub fn create(path: &Path) -> Result<ChannelDir, Error> {
        let owner = euid();
        let (pfd, name) = checked_parent(path, owner)?;
        rustix::fs::mkdirat(&pfd, &name, mode(0o700)).map_err(io)?;
        let fd = rustix::fs::openat(&pfd, &name, dir_flags(), Mode::empty()).map_err(|e| {
            Error::NotPrivate(format!(
                "the new folder was swapped before it was opened: {e}"
            ))
        })?;
        // Made 0700 so nothing passes through before it is checked; then set exactly, on the
        // handle (the umask can only remove bits).
        let st = rustix::fs::fstat(&fd).map_err(io)?;
        if st.st_uid != owner {
            return Err(Error::NotPrivate(
                "the new folder was swapped before it was opened".into(),
            ));
        }
        rustix::fs::fchmod(&fd, mode(FOLDER_MODE)).map_err(io)?;
        let st = rustix::fs::fstat(&fd).map_err(io)?;
        check_folder(st.st_uid, bits(st.st_mode), owner)?;
        Ok(ChannelDir {
            path: path.to_path_buf(),
            owner,
            fd,
        })
    }

    /// Checks an existing folder is a channel folder of this process's account: its parent
    /// first, then the folder itself, opened through the parent's handle without following a
    /// link.
    pub fn open(path: &Path) -> Result<ChannelDir, Error> {
        let owner = euid();
        let (pfd, name) = checked_parent(path, owner)?;
        let fd = rustix::fs::openat(&pfd, &name, dir_flags(), Mode::empty()).map_err(|e| {
            Error::NotPrivate(format!("cannot open it as a folder (not a link): {e}"))
        })?;
        let st = rustix::fs::fstat(&fd).map_err(io)?;
        check_folder(st.st_uid, bits(st.st_mode), owner)?;
        Ok(ChannelDir {
            path: path.to_path_buf(),
            owner,
            fd,
        })
    }

    /// Where it is.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The account that owns it and alone may write in it (this process's).
    pub fn owner(&self) -> u32 {
        self.owner
    }

    /// Its path still names the very folder this handle holds (not one swapped in), and that
    /// folder is still as it was made.
    fn path_is_still_it(&self) -> Result<(), Error> {
        let held = rustix::fs::fstat(&self.fd).map_err(io)?;
        let named = ChannelDir::open(&self.path)?;
        let now = rustix::fs::fstat(&named.fd).map_err(io)?;
        if (held.st_dev, held.st_ino) != (now.st_dev, now.st_ino) {
            return Err(Error::NotPrivate(
                "its path now names a different folder".into(),
            ));
        }
        check_folder(held.st_uid, bits(held.st_mode), self.owner)
    }
}

/// A mode as 32 bits (16 bits on macOS, 32 on Linux).
fn bits(mode: impl Into<u32>) -> u32 {
    mode.into()
}

fn io(e: rustix::io::Errno) -> Error {
    Error::Io(e.into())
}

/// Owned by `owner` (the server), mode exactly 0711: no one else may list, add, remove or
/// rename anything in it; no set-ID or sticky bits.
pub(crate) fn check_folder(uid: u32, mode: u32, owner: u32) -> Result<(), Error> {
    if uid != owner {
        return Err(Error::NotPrivate(format!(
            "owned by account {uid}, not {owner}"
        )));
    }
    if mode & 0o7777 != FOLDER_MODE {
        return Err(Error::NotPrivate(format!(
            "mode {:o}, not 711",
            mode & 0o7777
        )));
    }
    Ok(())
}

/// The parent belongs to root or the server's own account, and no one else can write to it
/// unless it is sticky (like /tmp, where only an entry's owner may rename or remove it).
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

/// The helper's end of the channel.
#[derive(Debug)]
pub struct SocketServer {
    listener: UnixListener,
    path: PathBuf,
}

impl SocketServer {
    /// Makes the socket `name` in `dir`, handed to `client` (the mover's user ID), mode 0600.
    /// [`Error::NameTaken`] if something of that name is already there: never reused.
    ///
    /// Binding takes a path; the folder's path is checked to name the held folder before and
    /// after. The entry is then confirmed to be this new socket, and its mode and owner are set
    /// relative to the folder's handle. Only this process's account can write in the folder,
    /// so the entry cannot be swapped between the check and the change.
    pub fn bind(dir: &ChannelDir, name: &str, client: u32) -> Result<SocketServer, Error> {
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
        if FileType::from_raw_mode(st.st_mode) != FileType::Socket || st.st_uid != euid() {
            return Err(Error::NotPrivate("the socket was swapped".into()));
        }
        rustix::fs::chmodat(&dir.fd, name, mode(SOCKET_MODE), AtFlags::empty()).map_err(io)?;
        if client != euid() {
            rustix::fs::chownat(
                &dir.fd,
                name,
                Some(rustix::fs::Uid::from_raw_unchecked(client)),
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

/// The mover's end: connects to the helper's socket at `path` and, before anything is sent,
/// checks the account on the other end is `server` (root, for the helper). A socket planted by
/// anyone else is refused with [`Error::UnexpectedPeer`].
pub fn connect(path: &Path, server: u32) -> Result<UnixStream, Error> {
    let stream = UnixStream::connect(path)?;
    let peer = caller_of(&stream)?;
    if peer.account != Account::Uid(server) {
        return Err(Error::UnexpectedPeer);
    }
    Ok(stream)
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
    fn only_the_owner_and_exactly_0711() {
        assert!(check_folder(0, 0o40711, 0).is_ok());
        assert!(check_folder(1000, 0o40711, 1000).is_ok());
        assert!(check_folder(1000, 0o40711, 0).is_err());
        for mode in [
            0o700, 0o710, 0o701, 0o751, 0o755, 0o777, 0o733, 0o713, 0o731, 0o611, 0o511, 0o311,
            0o1711, 0o2711, 0o4711,
        ] {
            assert!(check_folder(0, 0o40000 | mode, 0).is_err(), "{mode:o}");
        }
    }

    #[test]
    fn parent_must_not_let_others_rename_it() {
        assert!(check_parent(0, 0o755, 1000).is_ok());
        assert!(check_parent(1000, 0o700, 1000).is_ok());
        assert!(check_parent(0, 0o1777, 1000).is_ok());
        assert!(check_parent(1001, 0o755, 1000).is_err());
        assert!(check_parent(1001, 0o755, 0).is_err());
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
