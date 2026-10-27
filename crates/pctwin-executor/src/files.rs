//! Putting one settings file back only if it is still PCTwin's version (security review finding
//! 2, 26 October 2026; acceptance cases T23-017, T23-018 and T23-019: a person's later edit is
//! never lost).
//!
//! Checking a file and then writing over it leaves a moment between the two in which the person
//! (or their app) can save; the save would be lost. So the check and the replacement are one step
//! that cannot lose a save, in the shape Phase 1's file undo uses (the copy moved aside to a
//! private name, journaled first, and checked there):
//!
//! 1. The journal records the private name first ([`pctwin_journal::EffectUndo::Swapping`]), so
//!    after a crash undo knows where the file may be.
//! 2. The file is moved aside to that private name in one step that never replaces anything
//!    (Windows `MoveFileExW` without `MOVEFILE_REPLACE_EXISTING`; Linux `renameat2` with
//!    `RENAME_NOREPLACE`; macOS `renameatx_np` with `RENAME_EXCL`). From then on, an app that
//!    saves by name makes a new file under the name; nothing it writes lands in the moved copy.
//!    On Windows the file is first opened so that nobody may write it while it is held (and the
//!    open fails if someone is writing it now); the moved file is checked to be that very file.
//! 3. The moved copy is hashed. Only if it is exactly PCTwin's version is the earlier version
//!    written, under a second private name, flushed, and moved onto the name, again never
//!    replacing anything: if the person saved a new file meanwhile, theirs stays and the result
//!    is [`Swapped::Kept`].
//! 4. If the moved copy is not PCTwin's version (the person's), it is put back under its name;
//!    if a new file took the name meanwhile, it is put beside it as "<name> (kept by PCTwin)".
//!    Nothing of the person's is removed or written over.
//! 5. PCTwin's own version is removed last, after it is hashed once more.
//!
//! Every step can be run again after a crash: [`replace_if_unchanged`] first looks for the
//! private names and finishes from what it finds.
//!
//! What is not proven: on Linux and macOS a program that already had the file open (rather than
//! saving by name) can still write into the moved copy after it was hashed the second time and
//! before it is removed; that write would be lost with PCTwin's version. The window is the time of
//! one hash and one removal of a small settings file; Linux write leases (which Phase 1 uses) are
//! not taken here yet.

use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, Write};
use std::path::Path;

/// What putting one file back did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Swapped {
    /// The earlier version is under the name again (or, when there was none, PCTwin's file is
    /// gone).
    Restored,
    /// The file there was not PCTwin's version, or the person saved a new one while it was being
    /// put back: theirs is kept, untouched (beside the name only if a new file took the name).
    Kept,
    /// Nothing was under the name and nothing was aside: there was nothing to put back.
    Gone,
}

/// One file to put back: where it is moved aside, PCTwin's version's content hash, and the
/// earlier version's bytes (`None`: there was no file before).
#[derive(Debug, Clone, Copy)]
pub struct Swap<'a> {
    /// The private name, already in the journal: `.pctwin-undo-` and 32 hexadecimal digits.
    pub private: &'a str,
    pub ours: [u8; 32],
    pub earlier: Option<&'a [u8]>,
}

/// A moment between the steps, for tests to act in (a person saving).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Moment {
    /// Windows: the file is held under its name, about to be moved aside.
    Held,
    /// The file is under its private name, not yet hashed.
    Aside,
    /// It was hashed and is PCTwin's; the earlier version is about to be written.
    Checked,
    /// The earlier version is under its second private name, about to be moved onto the name.
    Written,
}

/// The content hash used throughout the journal (BLAKE3), of the whole file from its start.
fn hash_of(file: &mut File) -> io::Result<[u8; 32]> {
    file.seek(io::SeekFrom::Start(0))?;
    let mut h = blake3::Hasher::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(*h.finalize().as_bytes())
}

pub use pctwin_journal::is_private_name;

/// A new private name: `.pctwin-undo-` and 128 random bits from the system.
pub fn private_name() -> io::Result<String> {
    let mut bits = [0u8; 16];
    getrandom::fill(&mut bits).map_err(|_| io::Error::other("no random numbers"))?;
    Ok(format!(
        ".pctwin-undo-{}",
        bits.iter().map(|b| format!("{b:02x}")).collect::<String>()
    ))
}

fn invalid(what: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, what)
}

/// Puts the file `name` in the folder `dir` back as `swap` says, only if it is still PCTwin's
/// version; see the module notes. `moment` is called between the steps (tests). Safe to run again
/// after a crash with the same `swap`.
pub fn replace_if_unchanged(
    dir: &Path,
    name: &str,
    swap: &Swap<'_>,
    moment: &mut dyn FnMut(Moment),
) -> io::Result<Swapped> {
    if !is_private_name(swap.private) {
        return Err(invalid("not a private name"));
    }
    if name.is_empty() || name.contains(['/', '\\']) || name == "." || name == ".." {
        return Err(invalid("not a plain name"));
    }
    let new_name = format!("{}.new", swap.private);
    // A second private name left by a crash holds only PCTwin's earlier bytes, part written:
    // removed, and written again below if needed.
    remove_if_there(&dir.join(&new_name))?;
    let aside = dir.join(swap.private);
    let mut held = match open_held(&aside) {
        // A crash left it aside: finish from here.
        Ok(file) => file,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            match move_aside(dir, name, swap.private, moment)? {
                Some(file) => file,
                None => return Ok(Swapped::Gone),
            }
        }
        Err(e) => return Err(e),
    };
    moment(Moment::Aside);
    if hash_of(&mut held)? != swap.ours {
        drop(held);
        put_back(dir, swap.private, name)?;
        return Ok(Swapped::Kept);
    }
    moment(Moment::Checked);
    let mut taken = false;
    if let Some(bytes) = swap.earlier {
        let mut out = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(dir.join(&new_name))?;
        out.write_all(bytes)?;
        out.sync_all()?;
        drop(out);
        moment(Moment::Written);
        match move_no_replace(dir, &new_name, name) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                // The person saved a new file meanwhile: theirs stays.
                taken = true;
                std::fs::remove_file(dir.join(&new_name))?;
            }
            Err(e) => return Err(e),
        }
    }
    // PCTwin's own version, hashed once more before it goes; anything else is put back.
    if hash_of(&mut held)? != swap.ours {
        drop(held);
        put_back(dir, swap.private, name)?;
        return Ok(Swapped::Kept);
    }
    drop(held);
    std::fs::remove_file(&aside)?;
    sync_dir(dir)?;
    Ok(if taken {
        Swapped::Kept
    } else {
        Swapped::Restored
    })
}

fn remove_if_there(path: &Path) -> io::Result<()> {
    match std::fs::remove_file(path) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

/// Puts the moved file back under `name`, or beside it if a new file took the name.
fn put_back(dir: &Path, private: &str, name: &str) -> io::Result<()> {
    match move_no_replace(dir, private, name) {
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
        other => return other,
    }
    for n in 1..100u32 {
        let beside = if n == 1 {
            format!("{name} (kept by PCTwin)")
        } else {
            format!("{name} (kept by PCTwin {n})")
        };
        match move_no_replace(dir, private, &beside) {
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            other => return other,
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "no free name beside it",
    ))
}

/// Moves `name` aside to `private` and opens it there; `None` when nothing is under `name`.
#[cfg(windows)]
fn move_aside(
    dir: &Path,
    name: &str,
    private: &str,
    moment: &mut dyn FnMut(Moment),
) -> io::Result<Option<File>> {
    // Held so nobody can write it while it moves; refused if somebody is writing it now.
    let held = match open_held(&dir.join(name)) {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    moment(Moment::Held);
    move_no_replace(dir, name, private)?;
    // The file now under the private name is the one held (not one swapped in meanwhile).
    let there = open_held(&dir.join(private))?;
    let (a, b) = (
        winapi_util::file::information(&held)?,
        winapi_util::file::information(&there)?,
    );
    if a.volume_serial_number() != b.volume_serial_number() || a.file_index() != b.file_index() {
        drop((held, there));
        put_back(dir, private, name)?;
        return Err(io::Error::other("another file took its place"));
    }
    drop(there);
    Ok(Some(held))
}

#[cfg(unix)]
fn move_aside(
    dir: &Path,
    name: &str,
    private: &str,
    _: &mut dyn FnMut(Moment),
) -> io::Result<Option<File>> {
    match move_no_replace(dir, name, private) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    }
    open_held(&dir.join(private)).map(Some)
}

/// Opens a file to read, never following a link; Windows: others may only read it (and the
/// moving), so nobody writes it while it is held.
#[cfg(windows)]
fn open_held(path: &Path) -> io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_SHARE_READ: u32 = 0x1;
    const FILE_SHARE_DELETE: u32 = 0x4;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    let f = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)?;
    plain(&f)?;
    Ok(f)
}

#[cfg(unix)]
fn open_held(path: &Path) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    let f = OpenOptions::new()
        .read(true)
        .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
        .open(path)?;
    plain(&f)?;
    Ok(f)
}

fn plain(f: &File) -> io::Result<()> {
    let meta = f.metadata()?;
    #[cfg(windows)]
    let link = {
        use std::os::windows::fs::MetadataExt;
        meta.file_attributes() & 0x400 != 0
    };
    #[cfg(unix)]
    let link = meta.file_type().is_symlink();
    if meta.is_file() && !link {
        Ok(())
    } else {
        Err(invalid("not a plain file"))
    }
}

/// Moves `from` to `to` in `dir`, never replacing anything (`AlreadyExists` if `to` is taken).
#[cfg(windows)]
fn move_no_replace(dir: &Path, from: &str, to: &str) -> io::Result<()> {
    let mut source = tempfile::TempPath::try_from_path(dir.join(from))?;
    source.disable_cleanup(true);
    source.persist_noclobber(dir.join(to)).map_err(|e| {
        match e.error.raw_os_error() {
            // ERROR_FILE_EXISTS, ERROR_ALREADY_EXISTS.
            Some(80 | 183) => io::Error::new(io::ErrorKind::AlreadyExists, "taken"),
            _ => e.error,
        }
    })
}

#[cfg(unix)]
fn move_no_replace(dir: &Path, from: &str, to: &str) -> io::Result<()> {
    use rustix::fs::{CWD, RenameFlags, renameat_with};
    renameat_with(
        CWD,
        dir.join(from),
        CWD,
        dir.join(to),
        RenameFlags::NOREPLACE,
    )
    .map_err(io::Error::from)
}

#[cfg(unix)]
fn sync_dir(dir: &Path) -> io::Result<()> {
    File::open(dir)?.sync_all()
}

/// Windows flushes a folder's names with the files themselves (NTFS journals them).
#[cfg(windows)]
fn sync_dir(_: &Path) -> io::Result<()> {
    Ok(())
}
