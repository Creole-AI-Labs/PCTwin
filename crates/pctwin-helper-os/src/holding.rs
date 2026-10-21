//! The holding area on the new laptop (Security Design part C, `StageFor`): where other
//! people's files wait during the move, until they are handed over. It is made fresh for each
//! move, with a random name, inside a folder only administrators can make things in, so nothing
//! anyone else planted is ever used; inside it, one folder for each person
//! ([`person_folder`]), so one person's files are never handed to another:
//!
//! - Windows: `%ProgramFiles%\PCTwin\staging\<random>`, administrators only. In each person's
//!   folder the system and administrators have full control; the mover's account may read and
//!   write files there (the move lands them through
//!   the safety gate as the mover) but never change who may, not even for files it made
//!   ("owner rights" is given the same plain rights, which takes away an owner's own right to
//!   change the access list).
//! - macOS: `/Library/Application Support/PCTwin/staging/<random>`; Linux:
//!   `/var/lib/pctwin/staging/<random>`. These belong to root and only root writes in them
//!   (the move's folder can be passed through, not listed); each person's folder inside belongs
//!   to the mover for the move, readable by no one else, and is given to that person at the
//!   hand-over.

use std::path::PathBuf;

use crate::{Account, Error};

#[cfg(unix)]
use std::path::Path;

/// The folder every move's holding area is made in on this laptop.
pub fn staging_root() -> Option<PathBuf> {
    #[cfg(windows)]
    return std::env::var_os("ProgramFiles")
        .map(|p| PathBuf::from(p).join("PCTwin").join("staging"));
    #[cfg(target_os = "macos")]
    return Some(PathBuf::from("/Library/Application Support/PCTwin/staging"));
    #[cfg(all(unix, not(target_os = "macos")))]
    return Some(PathBuf::from("/var/lib/pctwin/staging"));
    #[cfg(not(any(windows, unix)))]
    None
}

/// Makes this move's holding area for the mover's account, and returns where it is. Needs an
/// administrator's rights. The people's folders inside it are made with [`person_folder`].
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
    // The move's own folder: administrators only; the mover writes only inside each person's.
    let _ = sid;
    crate::win::create_folder_with_access(&path, ADMIN_ONLY)?;
    Ok(path)
}

/// System and administrators full; the mover read, write, run and delete (0x1301bf), never the
/// right to change who may (no WRITE_DAC or WRITE_OWNER), and owner rights the same, which takes
/// away an owner's own right to change it for files the mover makes.
#[cfg(windows)]
fn mover_access(sid: &crate::Sid) -> String {
    format!(
        "D:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;OICI;0x1301bf;;;{})(A;OICI;0x1301bf;;;OW)",
        sid.as_str()
    )
}

/// Makes the folder for one person's files, `key` (1 to 64 of `a-z`, `A-Z`, `0-9` and `-`),
/// inside this move's holding area `area`, for the mover to fill. Never one that exists. On
/// Windows the person (`person`, when known) may read it too, for the hand-over at their sign-in.
pub fn person_folder(
    area: &std::path::Path,
    key: &str,
    mover: &Account,
    person: Option<&Account>,
) -> Result<PathBuf, Error> {
    if key.is_empty()
        || key.len() > 64
        || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        return Err(Error::BadInput("a person's folder key"));
    }
    #[cfg(windows)]
    {
        let Account::Sid(sid) = mover else {
            return Err(Error::BadInput("a Windows account"));
        };
        let path = area.join(key);
        let mut sddl = mover_access(sid);
        if let Some(Account::Sid(person)) = person {
            // Read, write and delete: their own files, handed over at their sign-in.
            sddl.push_str(&format!("(A;OICI;0x1301bf;;;{})", person.as_str()));
        }
        crate::win::create_folder_with_access(&path, &sddl)?;
        Ok(path)
    }
    #[cfg(unix)]
    {
        use rustix::fs::{Mode, OFlags};
        let Account::Uid(uid) = mover else {
            return Err(Error::BadInput("a Unix account"));
        };
        // The person is given the folder at the hand-over (`give_to`), not before.
        let _ = person;
        let flags = OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::RDONLY | OFlags::CLOEXEC;
        let parent =
            rustix::fs::open(area, flags, Mode::empty()).map_err(|e| Error::Io(e.into()))?;
        let st = rustix::fs::fstat(&parent).map_err(|e| Error::Io(e.into()))?;
        if st.st_uid != 0 || st.st_mode & 0o022 != 0 {
            return Err(Error::NotPrivate(
                "the holding area is not root's alone".into(),
            ));
        }
        rustix::fs::mkdirat(&parent, key, Mode::from_raw_mode(0o700))
            .map_err(|e| Error::Io(e.into()))?;
        let fd = rustix::fs::openat(&parent, key, flags, Mode::empty())
            .map_err(|e| Error::Io(e.into()))?;
        rustix::fs::fchown(&fd, Some(rustix::fs::Uid::from_raw(*uid)), None)
            .map_err(|e| Error::Io(e.into()))?;
        rustix::fs::fchmod(&fd, Mode::from_raw_mode(0o700)).map_err(|e| Error::Io(e.into()))?;
        Ok(area.join(key))
    }
    #[cfg(not(any(windows, unix)))]
    Err(Error::NotSupported("a holding area"))
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
    let st = rustix::fs::fstat(&fd).map_err(|e| Error::Io(e.into()))?;
    if st.st_uid != 0 {
        return Err(Error::NotPrivate("the new holding area was swapped".into()));
    }
    // Passed through to each person's folder, never listed.
    rustix::fs::fchmod(&fd, Mode::from_raw_mode(0o711)).map_err(|e| Error::Io(e.into()))?;
    let _ = uid;
    Ok(base.join(name))
}

/// The home folder of `account` (named `name`): Windows makes it now if the person never signed
/// in; Mac and Linux read it from the system's own records.
pub fn home_folder(account: &Account, name: &str) -> Result<PathBuf, Error> {
    #[cfg(windows)]
    {
        let Account::Sid(sid) = account else {
            return Err(Error::BadInput("a Windows account"));
        };
        crate::win::profile_folder(sid.as_str(), name)
    }
    #[cfg(unix)]
    {
        let _ = name;
        let Account::Uid(uid) = account else {
            return Err(Error::BadInput("a Unix account"));
        };
        passwd_entry(*uid).map(|(_, home)| home)
    }
    #[cfg(not(any(windows, unix)))]
    Err(Error::NotSupported("home folders"))
}

/// The primary group and home folder the system records for `uid`.
#[cfg(unix)]
pub fn passwd_entry(uid: u32) -> Result<(u32, PathBuf), Error> {
    #[cfg(target_os = "macos")]
    {
        let user = std::process::Command::new("/usr/bin/dscl")
            .args([".", "-search", "/Users", "UniqueID", &uid.to_string()])
            .output()?;
        let name = String::from_utf8_lossy(&user.stdout)
            .split_whitespace()
            .next()
            .map(str::to_string)
            .ok_or(Error::NotSupported("no such account"))?;
        let read = |key: &str| -> Result<String, Error> {
            let out = std::process::Command::new("/usr/bin/dscl")
                .args([".", "-read", &format!("/Users/{name}"), key])
                .output()?;
            String::from_utf8_lossy(&out.stdout)
                .split_once(':')
                .map(|(_, v)| v.trim().to_string())
                .ok_or(Error::NotSupported("the account's record is incomplete"))
        };
        let gid = read("PrimaryGroupID")?
            .parse()
            .map_err(|_| Error::NotSupported("the account's group is not a number"))?;
        Ok((gid, PathBuf::from(read("NFSHomeDirectory")?)))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let passwd = std::fs::read_to_string("/etc/passwd")?;
        passwd
            .lines()
            .find_map(|l| {
                let f: Vec<&str> = l.split(':').collect();
                (f.len() >= 7 && f[2].parse() == Ok(uid))
                    .then(|| Some((f[3].parse().ok()?, PathBuf::from(f[5]))))
                    .flatten()
            })
            .ok_or(Error::NotSupported("no such account"))
    }
}

/// Mac and Linux: gives everything under `folder` (and it) to `uid` (group `gid`), by handle,
/// never through a link (a link is left as it is), so the person's own hand-over can read it.
/// `folder`'s parent must be root's and not writable by anyone else.
#[cfg(unix)]
pub fn give_to(folder: &Path, uid: u32, gid: u32) -> Result<(), Error> {
    use rustix::fs::{AtFlags, Mode, OFlags};
    let flags = OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::RDONLY | OFlags::CLOEXEC;
    let parent = folder
        .parent()
        .ok_or(Error::BadInput("a folder inside the staging folder"))?;
    let pfd = rustix::fs::open(parent, flags, Mode::empty()).map_err(|e| Error::Io(e.into()))?;
    let st = rustix::fs::fstat(&pfd).map_err(|e| Error::Io(e.into()))?;
    if st.st_uid != 0 || st.st_mode & 0o022 != 0 {
        return Err(Error::NotPrivate(
            "the holding area's parent is not root's alone".into(),
        ));
    }
    let name = folder
        .file_name()
        .ok_or(Error::BadInput("a folder inside the staging folder"))?;
    let fd =
        rustix::fs::openat(&pfd, name, flags, Mode::empty()).map_err(|e| Error::Io(e.into()))?;
    fn walk(dir: &rustix::fd::OwnedFd, uid: u32, gid: u32) -> Result<(), Error> {
        let (u, g) = (
            Some(rustix::fs::Uid::from_raw(uid)),
            Some(rustix::fs::Gid::from_raw(gid)),
        );
        rustix::fs::fchown(dir, u, g).map_err(|e| Error::Io(e.into()))?;
        let entries = rustix::fs::Dir::read_from(dir).map_err(|e| Error::Io(e.into()))?;
        let mut names = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| Error::Io(e.into()))?;
            let n = entry.file_name().to_bytes();
            if n != b"." && n != b".." {
                names.push(entry.file_name().to_owned());
            }
        }
        for n in names {
            let st = rustix::fs::statat(dir, &n, AtFlags::SYMLINK_NOFOLLOW)
                .map_err(|e| Error::Io(e.into()))?;
            let kind = rustix::fs::FileType::from_raw_mode(st.st_mode);
            if kind == rustix::fs::FileType::Directory {
                let child = rustix::fs::openat(
                    dir,
                    &n,
                    OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::RDONLY | OFlags::CLOEXEC,
                    Mode::empty(),
                )
                .map_err(|e| Error::Io(e.into()))?;
                walk(&child, uid, gid)?;
            } else if kind == rustix::fs::FileType::RegularFile {
                let file = rustix::fs::openat(
                    dir,
                    &n,
                    OFlags::NOFOLLOW | OFlags::RDONLY | OFlags::CLOEXEC,
                    Mode::empty(),
                )
                .map_err(|e| Error::Io(e.into()))?;
                rustix::fs::fchown(&file, u, g).map_err(|e| Error::Io(e.into()))?;
            }
        }
        Ok(())
    }
    walk(&fd, uid, gid)
}
