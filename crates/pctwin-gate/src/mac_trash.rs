//! macOS: the Trash a proven copy goes into on undo (decided 20 October 2026).
//!
//! The Trash for a file is on the file's own drive, so the copy is moved there by one rename and
//! never copied: the person's own Trash (`~/.Trash`) when the copy is on the same drive as their
//! home folder, otherwise the drive's Trash for this person (`.Trashes/<uid>` at the top of the
//! drive). Only folders that are really folders, belong to this person and that no one else can
//! write to are used; anything else, and a drive with no `.Trashes` at all (macOS makes it), means
//! there is no Trash here and the copy is kept, untouched.
//!
//! A person's folder `.Trashes/<uid>` that is missing is made the way macOS makes it: owned by
//! the person, readable and writable by them alone. `.Trashes` itself is never made: on a drive
//! where macOS has not made one, PCTwin cannot tell how the drive's owner wants it, so it keeps
//! the copy and says so.

use std::os::fd::OwnedFd;

use rustix::fs::{AtFlags, FileType, Mode, OFlags};
use rustix::io::Errno;

/// A Trash folder on the same drive as a copy, held open.
pub(crate) struct Trash {
    /// The folder names are moved into: the Trash itself, or (where macOS's privacy protection
    /// does not let the Trash be opened) the home folder that holds it.
    pub(crate) base: OwnedFd,
    /// The way from `base` to the Trash: empty, or `.Trash/`.
    prefix: &'static str,
    /// The Trash folder's absolute path, for the journal and the report.
    pub(crate) shown: String,
}

impl Trash {
    /// The name `to` in the Trash, relative to [`Trash::base`].
    pub(crate) fn entry(&self, to: &str) -> String {
        format!("{}{to}", self.prefix)
    }

    /// The absolute path of `to` in the Trash.
    pub(crate) fn at(&self, to: &str) -> String {
        format!("{}/{to}", self.shown)
    }
}

/// How many names are tried in the Trash: Finder's numbers up to 99, then names with a random
/// tag (a Trash can hold hundreds of files of one name: "Untitled", say).
pub(crate) const MOST_TRASH_TRIES: u32 = crate::siblings::MOST_TRIES + 16;

/// The `n`th name tried for `name` in the Trash (from 1): the name itself, then the way Finder
/// numbers a clash, `report 2.docx`, `report 3.docx` up to 99, then `report 5f0c9a2e.docx` with
/// 32 random bits; never longer than a drive allows.
pub(crate) fn trash_name(name: &str, n: u32) -> String {
    if n <= 1 {
        name.to_string()
    } else if n <= crate::siblings::MOST_TRIES {
        crate::siblings::sibling_name(name, "", n, 0)
    } else {
        let mut bits = [0u8; 4];
        let tag = match getrandom::fill(&mut bits) {
            Ok(()) => bits.iter().map(|b| format!("{b:02x}")).collect::<String>(),
            Err(_) => format!("{n}"),
        };
        crate::siblings::sibling_name(name, &format!(" {tag}"), 1, 9)
    }
}

/// The Trash on the drive of the file `held`, if there is one PCTwin may use (see the module
/// notes). `Ok(None)` means there is none here: the copy is kept.
pub(crate) fn find(held: &std::fs::File) -> std::io::Result<Option<Trash>> {
    let dev = rustix::fs::fstat(held)?.st_dev;
    let uid = rustix::process::getuid().as_raw();
    if let Some(home) = crate::trash_home()
        && let Some(trash) = in_home(&home, dev, uid)?
    {
        return Ok(Some(trash));
    }
    on_drive(held, dev, uid)
}

/// Whether `st` is a real folder (never a link) on drive `dev`, owned by `uid`, that no one else
/// can write to.
fn private_folder(st: &rustix::fs::Stat, dev: rustix::fs::Dev, uid: u32) -> bool {
    FileType::from_raw_mode(st.st_mode) == FileType::Directory
        && st.st_dev == dev
        && st.st_uid == uid
        && st.st_mode & 0o022 == 0
}

/// Opens folder `name` in `dir` (the last part of `name` never followed if it is a link; a part
/// before it, `.Trashes`, was seen to be a real folder by the caller), making it first if it is
/// missing, with read, write and search for its owner alone. `None` unless it is a private
/// folder of `uid`'s on drive `dev`.
fn private_folder_in(
    dir: &OwnedFd,
    name: &str,
    dev: rustix::fs::Dev,
    uid: u32,
) -> std::io::Result<Option<OwnedFd>> {
    match rustix::fs::mkdirat(dir, name, Mode::RWXU) {
        Ok(()) | Err(Errno::EXIST) => {}
        Err(Errno::ACCESS | Errno::PERM | Errno::ROFS) => return Ok(None),
        Err(e) => return Err(e.into()),
    }
    let seen = match rustix::fs::statat(dir, name, AtFlags::SYMLINK_NOFOLLOW) {
        Ok(st) => st,
        Err(Errno::NOENT | Errno::ACCESS | Errno::PERM) => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    if !private_folder(&seen, dev, uid) {
        return Ok(None);
    }
    match rustix::fs::openat(
        dir,
        name,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    ) {
        Ok(fd) => {
            let opened = rustix::fs::fstat(&fd)?;
            Ok((opened.st_ino == seen.st_ino && private_folder(&opened, dev, uid)).then_some(fd))
        }
        Err(Errno::ACCESS | Errno::PERM | Errno::NOENT | Errno::LOOP | Errno::NOTDIR) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// The person's own Trash, `~/.Trash`, if their home folder is on drive `dev`.
fn in_home(
    home: &std::path::Path,
    dev: rustix::fs::Dev,
    uid: u32,
) -> std::io::Result<Option<Trash>> {
    let Some(home_str) = home.to_str() else {
        return Ok(None);
    };
    let Ok(h) = rustix::fs::open(
        home,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    ) else {
        return Ok(None);
    };
    if rustix::fs::fstat(&h)?.st_dev != dev {
        return Ok(None);
    }
    let shown = format!("{}/.Trash", home_str.trim_end_matches('/'));
    if let Some(fd) = private_folder_in(&h, ".Trash", dev, uid)? {
        return Ok(Some(Trash {
            base: fd,
            prefix: "",
            shown,
        }));
    }
    // macOS's privacy protection may refuse to open the Trash itself while still letting a name
    // be moved into it: then the move goes through the home folder, to `.Trash/<name>`, once the
    // Trash is seen to be a private folder of this person's (what lands there is checked after).
    match rustix::fs::statat(&h, ".Trash", AtFlags::SYMLINK_NOFOLLOW) {
        Ok(st) if private_folder(&st, dev, uid) => Ok(Some(Trash {
            base: h,
            prefix: ".Trash/",
            shown,
        })),
        _ => Ok(None),
    }
}

/// The drive's Trash for this person, `<top of the drive>/.Trashes/<uid>`, if macOS has made
/// `.Trashes` on the drive of `held` (drive `dev`).
fn on_drive(
    held: &std::fs::File,
    dev: rustix::fs::Dev,
    uid: u32,
) -> std::io::Result<Option<Trash>> {
    let st = rustix::fs::fstatfs(held)?;
    let top: Vec<u8> = st
        .f_mntonname
        .iter()
        .take_while(|c| **c != 0)
        .map(|c| c.to_ne_bytes()[0])
        .collect();
    let Ok(top) = String::from_utf8(top) else {
        return Ok(None);
    };
    if !top.starts_with('/') {
        return Ok(None);
    }
    let Ok(m) = rustix::fs::open(
        top.as_str(),
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    ) else {
        return Ok(None);
    };
    if rustix::fs::fstat(&m)?.st_dev != dev {
        return Ok(None);
    }
    // `.Trashes` must be macOS's own: a real folder on this drive (it is write-only for everyone,
    // with the sticky mark, so only its owner and each person's own folder inside matter).
    match rustix::fs::statat(&m, ".Trashes", AtFlags::SYMLINK_NOFOLLOW) {
        Ok(st)
            if FileType::from_raw_mode(st.st_mode) == FileType::Directory && st.st_dev == dev => {}
        _ => return Ok(None),
    }
    // `.Trashes` cannot be opened (no one may read it), only passed through: the person's folder
    // in it is reached from the top of the drive. Their folder itself is opened, never followed
    // if it is a link, and checked to be theirs alone on this very drive.
    let mine = uid.to_string();
    let Some(fd) = private_folder_in(&m, &format!(".Trashes/{mine}"), dev, uid)? else {
        return Ok(None);
    };
    let top = top.trim_end_matches('/');
    Ok(Some(Trash {
        base: fd,
        prefix: "",
        shown: format!("{top}/.Trashes/{mine}"),
    }))
}

/// Whether the copy `file` is in the Trash folder `trash` (an absolute path the journal
/// recorded) under `to`. A look the system refuses is "no".
pub(crate) fn holds(trash: &str, to: &str, file: crate::FileId) -> bool {
    let path = format!("{trash}/{to}");
    let Ok(fd) = rustix::fs::open(
        path.as_str(),
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::NOCTTY | OFlags::CLOEXEC,
        Mode::empty(),
    ) else {
        return false;
    };
    let f = std::fs::File::from(fd);
    rustix::fs::fstat(&f)
        .is_ok_and(|st| FileType::from_raw_mode(st.st_mode) == FileType::RegularFile)
        && crate::identity(&f).is_ok_and(|(id, _)| id == file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_in_the_trash_are_numbered_as_finder_does() {
        assert_eq!(trash_name("report.docx", 1), "report.docx");
        assert_eq!(trash_name("report.docx", 2), "report 2.docx");
        assert_eq!(trash_name("notes", 3), "notes 3");
        assert_eq!(trash_name(".bashrc", 2), ".bashrc 2");
        let long = format!("{}.txt", "é".repeat(125));
        assert_eq!(trash_name(&long, 1), long);
        let second = trash_name(&long, 2);
        assert!(
            second.len() <= 255 && second.ends_with(" 2.txt"),
            "{second}"
        );
        let tagged = trash_name("report.docx", crate::siblings::MOST_TRIES + 1);
        assert!(
            tagged.len() == "report 12345678.docx".len()
                && tagged.starts_with("report ")
                && tagged.ends_with(".docx"),
            "{tagged}"
        );
        assert_ne!(
            tagged,
            trash_name("report.docx", crate::siblings::MOST_TRIES + 1)
        );
        let long_tagged = trash_name(&long, MOST_TRASH_TRIES);
        assert!(long_tagged.len() <= 255, "{long_tagged}");
    }
}
