//! The holding area on the new laptop (Security Design part C, `StageFor`): where another
//! person's files wait during the move, until they are handed over. It is made fresh for each
//! move, with a random name, inside a folder only administrators can make things in, so nothing
//! anyone else planted is ever used:
//!
//! - Windows: `%ProgramFiles%\PCTwin\staging\<random>`. The system and administrators have full
//!   control; the mover's account may read and write files there (the move lands them through
//!   the safety gate as the mover) but never change who may, not even for files it made
//!   ("owner rights" is given the same plain rights, which takes away an owner's own right to
//!   change the access list).
//! - macOS: `/Library/Application Support/PCTwin/staging/<random>`; Linux:
//!   `/var/lib/pctwin/staging/<random>`. The parents belong to root and only root writes in
//!   them; the holding folder itself belongs to the mover for the move, readable by no one else.

use std::path::PathBuf;

use crate::{Account, Error};

/// Makes this move's holding area for the mover's account, and returns where it is. Needs an
/// administrator's rights.
pub fn create_holding_area(mover: &Account) -> Result<PathBuf, Error> {
    let name = crate::random_token()?;
    #[cfg(windows)]
    return windows(mover, &name);
    #[cfg(unix)]
    return unix(mover, &name);
    #[cfg(not(any(windows, unix)))]
    Err(Error::NotSupported("a holding area"))
}

/// System and administrators only, protected from what the folder above allows.
#[cfg(windows)]
const ADMIN_ONLY: &str = "D:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)";

#[cfg(windows)]
fn windows(mover: &Account, name: &str) -> Result<PathBuf, Error> {
    let Account::Sid(sid) = mover else {
        return Err(Error::BadInput("a Windows account"));
    };
    let program_files = std::env::var_os("ProgramFiles")
        .map(PathBuf::from)
        .ok_or(Error::NotSupported("no Program Files folder"))?;
    let mut path = program_files;
    for part in ["PCTwin", "staging"] {
        path.push(part);
        match crate::win::create_folder_with_access(&path, ADMIN_ONLY) {
            Ok(()) => {}
            // Made by an earlier move: only an administrator can make anything here. It must be a
            // real folder, never a link.
            Err(Error::NameTaken) => {
                let meta = std::fs::symlink_metadata(&path)?;
                if !meta.is_dir() || meta.file_type().is_symlink() {
                    return Err(Error::NotPrivate(format!(
                        "{} is not a plain folder",
                        path.display()
                    )));
                }
            }
            Err(e) => return Err(e),
        }
    }
    path.push(name);
    // 0x1301bf: read, write, run and delete, never change who may (no WRITE_DAC, WRITE_OWNER).
    let sddl = format!(
        "D:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;OICI;0x1301bf;;;{})(A;OICI;0x1301bf;;;OW)",
        sid.as_str()
    );
    crate::win::create_folder_with_access(&path, &sddl)?;
    Ok(path)
}

#[cfg(unix)]
fn unix(mover: &Account, name: &str) -> Result<PathBuf, Error> {
    use rustix::fs::{Mode, OFlags};
    let Account::Uid(uid) = mover else {
        return Err(Error::BadInput("a Unix account"));
    };
    let base = if cfg!(target_os = "macos") {
        PathBuf::from("/Library/Application Support/PCTwin/staging")
    } else {
        PathBuf::from("/var/lib/pctwin/staging")
    };
    // The parents: root's, only root writes in them, others may pass through to the holding
    // folder but not list it.
    let mut made = PathBuf::from("/");
    for part in base.iter().skip(1) {
        made.push(part);
        match std::fs::symlink_metadata(&made) {
            Ok(meta) if meta.is_dir() && !meta.file_type().is_symlink() => {}
            Ok(_) => {
                return Err(Error::NotPrivate(format!(
                    "{} is not a plain folder",
                    made.display()
                )));
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                std::fs::create_dir(&made)?;
            }
            Err(e) => return Err(e.into()),
        }
    }
    let dir_flags = OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::RDONLY | OFlags::CLOEXEC;
    let parent =
        rustix::fs::open(&base, dir_flags, Mode::empty()).map_err(|e| Error::Io(e.into()))?;
    let st = rustix::fs::fstat(&parent).map_err(|e| Error::Io(e.into()))?;
    if st.st_uid != 0 {
        return Err(Error::NotPrivate("the staging folder is not root's".into()));
    }
    rustix::fs::fchmod(&parent, Mode::from_raw_mode(0o711)).map_err(|e| Error::Io(e.into()))?;
    rustix::fs::mkdirat(&parent, name, Mode::from_raw_mode(0o700))
        .map_err(|e| Error::Io(e.into()))?;
    let fd = rustix::fs::openat(&parent, name, dir_flags, Mode::empty())
        .map_err(|e| Error::Io(e.into()))?;
    rustix::fs::fchown(&fd, Some(rustix::fs::Uid::from_raw(*uid)), None)
        .map_err(|e| Error::Io(e.into()))?;
    rustix::fs::fchmod(&fd, Mode::from_raw_mode(0o700)).map_err(|e| Error::Io(e.into()))?;
    Ok(base.join(name))
}
