//! Filling in a file under its real name, for a drive with no safe rename (exFAT on a Mac;
//! security design notes, "Drives with no safe rename", the user's decision of 15 October 2026).
//!
//! The drive offers neither a hard link nor a move it refuses when the name exists, so the file
//! is never placed with a rename at all:
//! 1. The journal records the step first (`Filling`, with the tag below and how the file is proven
//!    PCTwin's).
//! 2. The real name is created only if nothing has it (an exclusive create, which never follows a
//!    link). Something already there: the name is taken, and nothing of it is touched.
//! 3. At once: the hidden mark (made hidden as it is created on Windows, `fchflags` through its
//!    handle on a Mac), the tag (a creation time no ordinary file has, which the drive keeps
//!    exactly), the download marker; then its identity is recorded. A drive that cannot hide a
//!    file, or keeps nothing that proves it PCTwin's after a restart, is refused when it is
//!    chosen.
//! 4. It is filled from the checked temporary file, through the handle held on it, without ever
//!    setting its size first (so it never holds bytes that were not copied).
//! 5. Made durable (`F_FULLFSYNC` where the system has it, else a flush), then read back and
//!    checked against the whole file's fingerprint. The read-back is what the system returns, which
//!    may come from its memory rather than the drive; that is all any program can check.
//! 6. Before it is recorded `Filled`, PCTwin checks the name still holds its own file (the same
//!    file, with its tag and exactly the bytes copied). Another program that put its own file there
//!    meanwhile is never touched: its file stays, and PCTwin's checked temporary file is kept.
//! 7. Recorded `Filled`, and committed while still hidden and tagged, with what showing it
//!    needs kept in the journal. Only then is it shown ([`Destination::finish_show`]): the hidden
//!    mark cleared, its modified time set, its real creation date given back (when PCTwin's copy
//!    was made: the tag is never left on the person's file); and it is recorded shown. Every step
//!    before the commit still has the tag as its proof, and every step after it is finished by
//!    recovery from the journal, so no crash leaves a whole file that nothing proves PCTwin's.
//!
//! On a failure before step 7, PCTwin's own partial copy is removed only while the name still
//! holds it (checked as in step 6) and its bytes are exactly the bytes PCTwin copied (another
//! program may have written into it), and that is recorded; anything else is left, for recovery
//! to decide and report. After a crash the journal's recovery table decides (finish, remove, or
//! leave and report).

use std::io::{self, Read, Seek, SeekFrom, Write};
use std::sync::Arc;

use cap_std::fs::{Dir, OpenOptions};

use crate::{Born, DownloadMark, FileId, GateError, identity, marks};

pub use pctwin_journal::{Fill, FillTag, Proof, Show};

/// What is recorded before a file is filled in: the tag it gets at once, and how it is proven
/// PCTwin's after a crash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FillPlan {
    pub tag: FillTag,
    pub proof: Proof,
}

/// The journal's steps of a fill, recorded as they happen (see the module notes).
pub trait FillSteps {
    /// The file under its real name exists, as `made`: recorded before anything is copied into
    /// it.
    fn made(&mut self, made: FileId) -> io::Result<()>;
    /// Whether the file, now durable, reads back as exactly the whole checked file (its
    /// fingerprint). Given a reader at its start, past this laptop's cache where the drive allows
    /// it (as every received file is read back).
    fn read_back(&mut self, file: &mut dyn Read) -> io::Result<bool>;
    /// It is complete, durable and read back: recorded before its time is set and it is shown.
    fn filled(&mut self) -> io::Result<()>;
    /// PCTwin's own partial copy was removed from the real name after a failure.
    fn cleared(&mut self) -> io::Result<()>;
}

/// Bytes copied at a time.
const CHUNK: usize = 1 << 20;

/// For tests: what is copied at a time, and what runs after each piece (with the bytes copied so
/// far); an error from it stops the copy as a failing drive would.
#[derive(Clone)]
pub(crate) struct FillHook {
    pub(crate) chunk: usize,
    pub(crate) after_chunk: Arc<dyn Fn(u64) -> io::Result<()> + Send + Sync>,
}

/// The creation time PCTwin tests a drive with: 3 July 2001, 04:05:06 UTC. Real drives in CI
/// (15 October 2026) showed what a Mac keeps: FAT32 two-second steps (rounded down), exFAT whole
/// seconds until the drive is mounted again (hundredths after), so only an even whole second is
/// kept exactly on both, before and after a remount. FAT keeps local time, so tags stay out of the
/// months when clocks change anywhere.
pub(crate) const PROBE_BORN: Born = Born {
    secs: 994_133_106,
    nanos: 0,
};

/// Days from 1970-01-01 to the given date (proleptic Gregorian calendar).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The months a tag may fall in: June, July, December and January, when no place changes its
/// clocks (they change in March to May and September to November), so local time on a FAT drive
/// converts back exactly.
const TAG_MONTHS: [i64; 4] = [1, 6, 7, 12];

/// A fresh tag: a creation time on an even whole second (which FAT and exFAT hold exactly,
/// before and after a remount), on a day of 1981 to 2099 in [`TAG_MONTHS`], chosen at random
/// (about 2^29 values). No ordinary file made today has one; it is also read back right after it
/// is set, and a drive that did not keep it is never filled.
pub(crate) fn fresh_born() -> Born {
    use std::hash::{BuildHasher, RandomState};
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let r = RandomState::new().hash_one((std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
    #[allow(
        clippy::cast_possible_wrap,
        reason = "each part is reduced far below i64's range first"
    )]
    let pick = |shift: u32, n: u64| ((r >> shift) % n) as i64;
    let year = 1981 + pick(0, 119);
    let month = TAG_MONTHS[usize::try_from(pick(8, 4)).unwrap_or(0)];
    let day = 1 + pick(12, 28);
    let second = 2 * pick(20, 43_200);
    Born {
        secs: days_from_civil(year, month, day) * 86_400 + second,
        nanos: 0,
    }
}

fn as_time(born: Born) -> io::Result<std::time::SystemTime> {
    let secs = u64::try_from(born.secs).map_err(|_| io::Error::other("a time before 1970"))?;
    Ok(std::time::UNIX_EPOCH + std::time::Duration::new(secs, born.nanos))
}

/// Gives the open file `file` the creation time `born`, through its handle.
pub(crate) fn set_born(file: &std::fs::File, born: Born) -> io::Result<()> {
    set_created(file, as_time(born)?)
}

/// Gives the open file `file` the creation time `at`, through its handle.
pub(crate) fn set_created(file: &std::fs::File, at: std::time::SystemTime) -> io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        use std::os::macos::fs::FileTimesExt;
        file.set_times(std::fs::FileTimes::new().set_created(at))
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::FileTimesExt;
        file.set_times(std::fs::FileTimes::new().set_created(at))
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        let _ = (file, at);
        Err(io::ErrorKind::Unsupported.into())
    }
}

/// Whether this system can mark a file hidden (macOS: by name, checked through its handle;
/// Windows: as it is made). Linux has no hidden mark.
pub(crate) const CAN_HIDE: bool = cfg!(any(target_os = "macos", windows));

/// Another file is at the name of the file PCTwin is filling in or showing: the name is refused,
/// and that file is never filled, shown or removed.
#[derive(Debug)]
pub(crate) struct NameReplaced;

impl std::fmt::Display for NameReplaced {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("another file was put at the name PCTwin was using")
    }
}

impl std::error::Error for NameReplaced {}

/// Whether `e` says another file was put at the name.
pub(crate) fn is_replaced(e: &io::Error) -> bool {
    e.get_ref().is_some_and(|inner| inner.is::<NameReplaced>())
}

/// macOS: the hidden mark (`UF_HIDDEN`).
#[cfg(target_os = "macos")]
const UF_HIDDEN: u32 = 0x0000_8000;

/// macOS: the open file's flags, read through its own handle.
#[cfg(target_os = "macos")]
fn flags_of(file: &std::fs::File) -> io::Result<u32> {
    use std::os::macos::fs::MetadataExt;
    Ok(file.metadata()?.st_flags())
}

/// macOS: whether the name `name` in `dir` holds the very file `file` PCTwin holds open (a plain
/// file, not a link, with the same identity), and PCTwin's file is still linked: on FAT and
/// exFAT a file's number is the place of its entry, which a file made where a removed one was
/// can take.
#[cfg(target_os = "macos")]
fn name_holds(dir: &Dir, name: &str, file: &std::fs::File) -> io::Result<bool> {
    use std::os::unix::fs::MetadataExt;
    match dir.symlink_metadata(name) {
        Ok(meta) if meta.is_file() => {}
        Ok(_) => return Ok(false),
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e),
    }
    if file.metadata()?.nlink() == 0 {
        return Ok(false);
    }
    let there = dir.open(name)?.into_std();
    Ok(identity(&there)?.0 == identity(file)?.0)
}

/// macOS: marks the file PCTwin holds open as `file`, at `name` in `dir`, hidden, or clears the
/// mark, changing no other flag. No safe call sets a file's flags through its handle or without
/// following a link (nix has only `chflags`), so it is set by name, as close to the checks as can
/// be: the name must hold this very file (not a link, still linked); then the folder's path is
/// read from the folder's own handle, and the file at that exact path is looked at without
/// following a link and must be PCTwin's file; its own flags, with only the hidden mark changed,
/// are set; then the mark is read back through the handle, and must be as asked. Another file at
/// the name refuses it ([`NameReplaced`]). `hook` (tests) runs before the checks
/// (`<op>:named`), between the name check and the look at the exact path (`<op>:held`), and just
/// before the call (`<op>:checked`), where `op` is `hide` or `show`.
#[cfg(target_os = "macos")]
fn set_hidden_named(
    dir: &Dir,
    name: &str,
    file: &std::fs::File,
    hidden: bool,
    hook: Option<&crate::ShowHook>,
) -> io::Result<()> {
    use std::os::unix::ffi::OsStrExt;
    let op = if hidden { "hide" } else { "show" };
    let step = |at: &str| match hook {
        Some(hook) => hook(&format!("{op}:{at}")),
        None => Ok(()),
    };
    if name.contains('/') || name.is_empty() || name == "." || name == ".." {
        return Err(io::Error::from(io::ErrorKind::InvalidInput));
    }
    step("named")?;
    if !name_holds(dir, name, file)? {
        return Err(io::Error::other(NameReplaced));
    }
    step("held")?;
    // The folder's path now, from its own handle, and the file at that exact path, not followed
    // if it is a link: it must be PCTwin's very file, and only its own hidden mark changes.
    let folder = rustix::fs::getpath(dir)?;
    let path = std::path::Path::new(std::ffi::OsStr::from_bytes(folder.as_bytes())).join(name);
    let flags = {
        use std::os::macos::fs::MetadataExt;
        let there = std::fs::symlink_metadata(&path)?;
        let held = file.metadata()?;
        if !there.file_type().is_file()
            || there.st_dev() != held.st_dev()
            || there.st_ino() != held.st_ino()
            || held.st_nlink() == 0
        {
            return Err(io::Error::other(NameReplaced));
        }
        if hidden {
            there.st_flags() | UF_HIDDEN
        } else {
            there.st_flags() & !UF_HIDDEN
        }
    };
    step("checked")?;
    nix::unistd::chflags(&path, nix::sys::stat::FileFlag::from_bits_retain(flags))
        .map_err(io::Error::from)?;
    if (flags_of(file)? & UF_HIDDEN != 0) == hidden {
        Ok(())
    } else if name_holds(dir, name, file)? {
        Err(io::Error::other(if hidden {
            "the drive did not keep the hidden mark"
        } else {
            "the drive did not clear the hidden mark"
        }))
    } else {
        Err(io::Error::other(NameReplaced))
    }
}

/// Windows' hidden attribute.
#[cfg(windows)]
pub(crate) const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;

/// Whether the open file `file` is marked hidden.
pub(crate) fn is_hidden(file: &std::fs::File) -> io::Result<bool> {
    #[cfg(target_os = "macos")]
    {
        Ok(flags_of(file)? & UF_HIDDEN != 0)
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        Ok(file.metadata()?.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0)
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        let _ = file;
        Ok(false)
    }
}

/// Marks the file PCTwin holds open as `file`, at `name` in `dir`, hidden (on a Mac by name,
/// checked through the handle: [`set_hidden_named`]; on Windows it was made hidden as it was
/// created: this checks it is), and fails unless it reads back hidden through the handle.
pub(crate) fn hide(
    dir: &Dir,
    name: &str,
    file: &std::fs::File,
    hook: Option<&crate::ShowHook>,
) -> io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        set_hidden_named(dir, name, file, true, hook)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (dir, name, hook);
        if is_hidden(file)? {
            Ok(())
        } else {
            Err(io::Error::other("the drive did not keep the hidden mark"))
        }
    }
}

/// Clears the hidden mark on the file PCTwin holds open as `file`, at `name` in `dir`: on a Mac
/// by name, checked through the handle ([`set_hidden_named`]); on Windows through the handle, by
/// giving it the attributes of `like` (one of PCTwin's own plain files, never hidden); and fails
/// unless it reads back shown through the handle.
pub(crate) fn show_file(
    dir: &Dir,
    name: &str,
    file: &std::fs::File,
    like: &std::fs::File,
    hook: Option<&crate::ShowHook>,
) -> io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        let _ = like;
        set_hidden_named(dir, name, file, false, hook)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (dir, name, hook);
        #[cfg(windows)]
        file.set_permissions(like.metadata()?.permissions())?;
        #[cfg(not(windows))]
        let _ = like;
        if is_hidden(file)? {
            Err(io::Error::other("the drive did not clear the hidden mark"))
        } else {
            Ok(())
        }
    }
}

/// Whether the drive under `dir` can hide a file: tried on the probe file `name` (PCTwin's own),
/// which is shown again afterwards, each read back through its handle. Windows hides a file as it is made,
/// which every drive it writes takes; each file is read back hidden all the same.
pub(crate) fn hides(dir: &Dir, name: &str) -> bool {
    #[cfg(target_os = "macos")]
    {
        let mut options = OpenOptions::new();
        options.read(true).write(true);
        dir.open_with(name, &options)
            .map(cap_std::fs::File::into_std)
            .is_ok_and(|f| {
                set_hidden_named(dir, name, &f, true, None).is_ok()
                    && set_hidden_named(dir, name, &f, false, None).is_ok()
            })
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (dir, name);
        cfg!(windows)
    }
}

/// The creation time of the open file `file`, as its drive keeps it.
pub(crate) fn born_of(file: &std::fs::File) -> Option<Born> {
    file.metadata().ok()?.created().ok().map(Born::of)
}

/// Whether this system can set a file's creation time at all (the drive's test says whether the
/// drive keeps it exactly).
pub(crate) const CAN_SET_BORN: bool = cfg!(any(target_os = "macos", windows));

/// Whether the drive under `dir` keeps a creation time exactly as set: tried on the probe file
/// `name` (PCTwin's own), read back through a fresh handle.
pub(crate) fn keeps_born(dir: &Dir, name: &str) -> bool {
    if !CAN_SET_BORN {
        return false;
    }
    let mut options = OpenOptions::new();
    options.write(true);
    let set = dir
        .open_with(name, &options)
        .map(cap_std::fs::File::into_std)
        .and_then(|f| set_born(&f, PROBE_BORN));
    set.is_ok() && dir.open(name).ok().and_then(|f| born_of(&f.into_std())) == Some(PROBE_BORN)
}

/// Makes the file's writes durable on the drive itself: `F_FULLFSYNC` on a Mac (a plain flush
/// there may stop at the drive's own cache), else a flush.
pub(crate) fn full_sync(file: &std::fs::File) -> io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        if rustix::fs::fcntl_fullfsync(file).is_ok() {
            return Ok(());
        }
    }
    file.sync_all()
}

/// Where a fill had got to, for what a failure may undo.
pub(crate) struct Progress {
    /// The tag was given (so the name must still carry it to be PCTwin's).
    pub(crate) born: Option<Born>,
    /// Bytes copied into it.
    pub(crate) written: u64,
}

/// Whether the name `name` in `dir` still holds PCTwin's own file `target`: a regular file, the
/// very same file, carrying the tag it was given, with exactly the bytes copied.
pub(crate) fn still_ours(
    dir: &Dir,
    name: &str,
    target: &std::fs::File,
    progress: &Progress,
) -> io::Result<bool> {
    match dir.symlink_metadata(name) {
        Ok(meta) if meta.is_file() => {}
        Ok(_) => return Ok(false),
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e),
    }
    let there = dir.open(name)?.into_std();
    let meta = there.metadata()?;
    if !meta.is_file() || meta.len() != progress.written {
        return Ok(false);
    }
    if identity(&there)?.0 != identity(target)?.0 {
        return Ok(false);
    }
    Ok(progress.born.is_none() || born_of(&there) == progress.born)
}

/// Copies the whole held file `from` into `to` (both at their start), never setting `to`'s size
/// first; `progress.written` counts what was copied, and `hook` runs after each piece.
pub(crate) fn copy_held(
    from: &mut std::fs::File,
    to: &mut std::fs::File,
    progress: &mut Progress,
    hook: Option<&FillHook>,
) -> io::Result<()> {
    from.seek(SeekFrom::Start(0))?;
    to.seek(SeekFrom::Start(0))?;
    let mut buf = vec![0u8; hook.map_or(CHUNK, |h| h.chunk.max(1))];
    loop {
        let n = match from.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        to.write_all(&buf[..n])?;
        progress.written += n as u64;
        if let Some(hook) = hook {
            (hook.after_chunk)(progress.written)?;
        }
    }
    to.flush()
}

/// Puts the download marker on the file filled in, as on every received file.
pub(crate) fn put_mark(
    dir: &Dir,
    name: &str,
    file: &std::fs::File,
    mark: Option<DownloadMark>,
    tries: bool,
) -> Result<bool, GateError> {
    match mark {
        Some(mark) if tries => marks::put(dir, name, file, mark),
        Some(_) => Ok(false),
        None => Ok(true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_tags_are_even_seconds_in_months_without_clock_changes() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(
            days_from_civil(2001, 7, 3) * 86_400 + 4 * 3600 + 5 * 60 + 6,
            PROBE_BORN.secs
        );
        let tags: Vec<Born> = (0..2000).map(|_| fresh_born()).collect();
        for t in &tags {
            assert_eq!(t.nanos, 0, "{t:?}");
            assert_eq!(t.secs % 2, 0, "{t:?}");
            // 1981-01-01 to 2100-01-01.
            assert!((347_155_200..4_102_444_800).contains(&t.secs), "{t:?}");
            let days = t.secs.div_euclid(86_400);
            let month = (1..=12)
                .rev()
                .find(|m| {
                    (1981..2100).any(|y| {
                        let start = days_from_civil(y, *m, 1);
                        (start..start + 28).contains(&days)
                    })
                })
                .unwrap_or(0);
            assert!(TAG_MONTHS.contains(&month), "{t:?} in month {month}");
        }
        let distinct: std::collections::HashSet<_> = tags.iter().collect();
        assert!(distinct.len() > 1990, "{}", distinct.len());
        assert_eq!(PROBE_BORN.nanos, 0);
        assert_eq!(PROBE_BORN.secs % 2, 0);
    }
}

/// How finishing a filled-in file after a restart ended.
#[derive(Debug)]
pub enum FillFinish {
    /// Durable, read back whole, given its time and shown: here is what it landed as.
    Finished(crate::Stat),
    /// Nothing is at the name now.
    Gone,
    /// The file at the name is not proven PCTwin's now, or no longer reads back whole: left.
    NotProven,
}

/// How removing PCTwin's own partial copy after a restart ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillRemoval {
    /// Removed: it was proven PCTwin's, and its bytes exactly the start of the checked file.
    Removed,
    /// Nothing is at the name now.
    Gone,
    /// Not proven PCTwin's partial copy now: left as it is.
    NotProven,
}

/// Whether the open file `file` is the one PCTwin made for `fill`: its recorded identity where
/// the drive keeps identities, or the creation time it was given (the same rule as the journal's
/// recovery table).
fn proven_fill(file: &std::fs::File, fill: &Fill) -> io::Result<bool> {
    // Nothing is PCTwin's until the journal has recorded the file it made: what it meant to do
    // (the name, the tag it chose) proves nothing (the journal's rule exactly).
    let Some(made) = fill.made else {
        return Ok(false);
    };
    Ok(match fill.proof {
        Proof::StableId => made == identity(file)?.0,
        Proof::Token => fill.tag.born.is_some() && born_of(file) == fill.tag.born,
        Proof::SameMountId | Proof::None => false,
    })
}

/// The two open files' first `len` bytes are the same, and `a` is exactly `len` long.
pub(crate) fn same_start_of(
    a: &mut std::fs::File,
    b: &mut std::fs::File,
    len: u64,
) -> io::Result<bool> {
    if a.metadata()?.len() != len || b.metadata()?.len() < len {
        return Ok(false);
    }
    a.seek(SeekFrom::Start(0))?;
    b.seek(SeekFrom::Start(0))?;
    let (mut x, mut y) = (vec![0u8; 64 * 1024], vec![0u8; 64 * 1024]);
    let mut left = len;
    while left > 0 {
        let n = usize::try_from(left.min(x.len() as u64)).unwrap_or(x.len());
        a.read_exact(&mut x[..n])?;
        b.read_exact(&mut y[..n])?;
        if x[..n] != y[..n] {
            return Ok(false);
        }
        left -= n as u64;
    }
    Ok(true)
}

impl crate::Destination {
    /// The creation time of the regular file at the stored path `stored`, as its drive keeps it;
    /// `None` if there is none (or no file). Never follows a link.
    pub fn born(&self, stored: &str) -> io::Result<Option<Born>> {
        let Some((dir, name)) = self.open_stored_folder(stored)? else {
            return Ok(None);
        };
        match dir.symlink_metadata(name) {
            Ok(meta) if meta.is_file() => Ok(born_of(&dir.open(name)?.into_std())),
            Ok(_) => Ok(None),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Whether the file at the stored path `stored` is exactly `len` bytes, and they are exactly
    /// the first `len` bytes of the file at the stored path `temp`. Never follows a link.
    pub fn same_start(&self, stored: &str, temp: &str, len: u64) -> io::Result<bool> {
        let mut a = self.open_read(stored)?;
        let mut b = self.open_read(temp)?;
        same_start_of(&mut a, &mut b, len)
    }

    /// After a restart: finishes the file filled in at the stored path `stored` for `fill`, if it
    /// is proven PCTwin's right now: made durable, read back whole (`read_back`, given the file at
    /// its start), `filled` (record it, if not yet recorded), given `modified`, shown, and the
    /// folder flushed. Anything not proven is left exactly as it is.
    /// The order is the fill's own: shown, the real creation date `created` given back (when
    /// PCTwin's copy was made), the modified time `modified` set last.
    /// It stays hidden and tagged: commit it, then [`finish_show`](Self::finish_show).
    pub fn finish_fill(
        &self,
        stored: &str,
        fill: &Fill,
        read_back: impl FnOnce(&mut dyn Read) -> io::Result<bool>,
        filled: impl FnOnce() -> io::Result<()>,
    ) -> Result<FillFinish, GateError> {
        let Some((dir, name)) = self.open_stored_folder(stored)? else {
            return Ok(FillFinish::Gone);
        };
        match dir.symlink_metadata(name) {
            Ok(meta) if meta.is_file() => {}
            Ok(_) => return Ok(FillFinish::NotProven),
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(FillFinish::Gone),
            Err(e) => return Err(e.into()),
        }
        let mut options = OpenOptions::new();
        options.read(true).write(true);
        let file = dir.open_with(name, &options)?.into_std();
        if !proven_fill(&file, fill)? {
            return Ok(FillFinish::NotProven);
        }
        full_sync(&file)?;
        // Past the cache where the drive allows it; after a restart the file's pages are not
        // known to be kept out of memory, so the check says honestly where its bytes came from.
        let mut reader = self.read_back_of(&dir, name, &file, false)?;
        if !read_back(&mut reader)? {
            return Ok(FillFinish::NotProven);
        }
        drop(reader);
        filled()?;
        full_sync(&file)?;
        crate::flush_name(&dir, name);
        let meta = file.metadata()?;
        Ok(FillFinish::Finished(crate::Stat {
            len: meta.len(),
            modified: meta.modified().ok(),
            id: identity(&file)?.0,
        }))
    }

    /// After a restart: removes PCTwin's own partial copy at the stored path `stored` for `fill`,
    /// only if, right now, it is proven PCTwin's, exactly `len` bytes, and they are exactly the
    /// start of the checked temporary file at `temp`; the name is checked once more to hold that
    /// very file just before it is removed. Anything else is left.
    pub fn remove_fill(
        &self,
        stored: &str,
        fill: &Fill,
        temp: &str,
        len: u64,
    ) -> Result<FillRemoval, GateError> {
        let Some((dir, name)) = self.open_stored_folder(stored)? else {
            return Ok(FillRemoval::Gone);
        };
        match dir.symlink_metadata(name) {
            Ok(meta) if meta.is_file() => {}
            Ok(_) => return Ok(FillRemoval::NotProven),
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(FillRemoval::Gone),
            Err(e) => return Err(e.into()),
        }
        let mut held = dir.open(name)?.into_std();
        let mut source = self.open_read(temp)?;
        // PCTwin's by the file the journal recorded it made (its identity, or its tag), with at
        // least one byte, all of them the start of the checked file. An empty file holds nothing
        // of PCTwin's, and a lookalike could be another program's: never removed.
        if len == 0 || !proven_fill(&held, fill)? || !same_start_of(&mut held, &mut source, len)? {
            return Ok(FillRemoval::NotProven);
        }
        let progress = Progress {
            born: fill.tag.born.filter(|_| born_of(&held) == fill.tag.born),
            written: len,
        };
        if !still_ours(&dir, name, &held, &progress)? {
            return Ok(FillRemoval::NotProven);
        }
        drop(held);
        dir.remove_file(name)?;
        crate::sync_folder(&dir)?;
        Ok(FillRemoval::Removed)
    }
}

/// A fill with no journal ([`crate::Sealed::claim`]): nothing to record, and the read-back is
/// checked byte for byte against the checked file itself.
pub(crate) struct Unjournaled {
    pub(crate) source: std::fs::File,
}

impl FillSteps for Unjournaled {
    fn made(&mut self, _: FileId) -> io::Result<()> {
        Ok(())
    }
    fn read_back(&mut self, file: &mut dyn Read) -> io::Result<bool> {
        self.source.seek(SeekFrom::Start(0))?;
        let (mut x, mut y) = (vec![0u8; 64 * 1024], vec![0u8; 64 * 1024]);
        loop {
            let n = read_full(file, &mut x)?;
            let m = read_full(&mut self.source, &mut y)?;
            if n != m || x[..n] != y[..m] {
                return Ok(false);
            }
            if n == 0 {
                return Ok(true);
            }
        }
    }
    fn filled(&mut self) -> io::Result<()> {
        Ok(())
    }
    fn cleared(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Reads as much of `buf` as `r` has (less only at its end).
fn read_full(r: &mut dyn Read, buf: &mut [u8]) -> io::Result<usize> {
    let mut got = 0;
    while got < buf.len() {
        match r.read(&mut buf[got..]) {
            Ok(0) => break,
            Ok(n) => got += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(got)
}

/// One of PCTwin's own plain files in `dir`, made only for its attributes and removed at once
/// (held open: Windows removes it when the handle closes). Never hidden.
fn plain_file(dir: &Dir) -> io::Result<std::fs::File> {
    use std::hash::{BuildHasher, RandomState};
    let name = crate::temp_name(&format!(
        "plain-{:016x}",
        RandomState::new().hash_one(std::process::id())
    ));
    let mut options = OpenOptions::new();
    options.read(true).write(true).create_new(true);
    let file = dir.open_with(&name, &options)?.into_std();
    let _ = dir.remove_file(&name);
    Ok(file)
}

impl crate::Destination {
    /// Whether the regular file at the stored path `stored` is marked hidden (`false` if there
    /// is none). Never follows a link.
    pub fn hidden(&self, stored: &str) -> io::Result<bool> {
        let Some((dir, name)) = self.open_stored_folder(stored)? else {
            return Ok(false);
        };
        match dir.symlink_metadata(name) {
            Ok(meta) if meta.is_file() => is_hidden(&dir.open(name)?.into_std()),
            Ok(_) => Ok(false),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e),
        }
    }
}

/// Nanoseconds since 1970 (0 before it).
pub(crate) fn nanos_of(t: std::time::SystemTime) -> i64 {
    t.duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_nanos()).unwrap_or(i64::MAX))
}

/// The second check, beside its tag or recorded identity, that the file about to be shown is the
/// very one PCTwin filled in. Windows gives a file made at a name just freed the creation time of
/// the file that had it (file-system tunnelling, on NTFS and FAT), so a tag alone could pass
/// another program's file.
pub enum ShowCheck<'a> {
    /// In the program that filled it in: the very file PCTwin still holds open
    /// ([`crate::Claimed::show_check`]), compared as it is now. A number recorded earlier is
    /// never asked for: on FAT and exFAT a file's number changes as it fills or across a
    /// remount, and Linux gives a freed number to the next file at once.
    Held(&'a std::fs::File),
    /// After a restart: its bytes, read now, against the whole file's fingerprint.
    Bytes(&'a mut dyn FnMut(&mut dyn Read) -> io::Result<bool>),
}

/// How showing a committed, filled-in file ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shown {
    /// Proven PCTwin's, then shown, with its real dates: record it shown, with its identity as
    /// it is now (on a Mac the identity carries the creation date, just given back).
    Shown(FileId),
    /// Already shown (no longer hidden, the tag that goes last gone, and PCTwin's copy: the very
    /// file made, or after a restart the whole file's bytes): record it shown. With its identity
    /// now when it is still the very file PCTwin made (the same file number on the same drive);
    /// otherwise the identity recorded at the commit stays.
    AlreadyShown(Option<FileId>),
    /// Nothing is at its name now: nothing left to do; record it shown.
    Gone,
    /// Nothing proves it PCTwin's copy now (another program's, or changed), hidden or not: left
    /// as it is; record it shown, with a note.
    NotProven,
}

impl crate::Destination {
    /// Shows a committed file filled in under its real name at the stored path `stored`, as
    /// `show` records (after a restart too): proven PCTwin's first (its identity where the drive
    /// keeps identities, else its tag), then the hidden mark cleared, its modified time set, its
    /// real creation date given back (the tag goes last, so a file seen without it was already
    /// shown), flushed. Each step can be done again. A file not proven PCTwin's is never touched.
    pub fn finish_show(
        &self,
        stored: &str,
        show: &Show,
        check: ShowCheck<'_>,
    ) -> Result<Shown, GateError> {
        let Some((dir, name)) = self.open_stored_folder(stored)? else {
            return Ok(Shown::Gone);
        };
        match dir.symlink_metadata(name) {
            Ok(meta) if meta.is_file() => {}
            Ok(_) => return Ok(Shown::NotProven),
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Shown::Gone),
            Err(e) => return Err(e.into()),
        }
        let mut options = OpenOptions::new();
        options.read(true).write(true);
        let mut file = dir.open_with(name, &options)?.into_std();
        let now = identity(&file)?.0;
        // In the same run, the very file PCTwin holds, as it is now; after a restart, its bytes.
        let (mine, same) = match check {
            ShowCheck::Held(held) => {
                let same = identity(held)?.0 == now;
                (same, same)
            }
            ShowCheck::Bytes(whole) => {
                file.seek(SeekFrom::Start(0))?;
                (whole(&mut file)?, false)
            }
        };
        let proven = proven_fill(&file, &show.fill)? && mine;
        if !proven {
            // Its tag gone: already shown, but only if it is PCTwin's copy, not hidden, and the
            // very file PCTwin holds (in the same run), or the whole file's bytes (after a
            // restart). Anything else is left exactly as it is.
            let ours = !is_hidden(&file)? && mine;
            return Ok(if ours {
                Shown::AlreadyShown(same.then_some(now))
            } else {
                Shown::NotProven
            });
        }
        let step = |what: &str| match &self.show_hook {
            Some(hook) => hook(what),
            None => Ok(()),
        };
        if is_hidden(&file)? {
            let like = plain_file(&dir)?;
            show_file(&dir, name, &file, &like, self.flags_hook.as_ref())?;
        }
        step("shown")?;
        if let Some(ns) = show.modified_ns.and_then(|ns| u64::try_from(ns).ok()) {
            file.set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_nanos(ns))?;
        }
        step("modified")?;
        if show.fill.tag.born.is_some() {
            let created = match show.created {
                Some(b) => as_time(b)?,
                None => std::time::SystemTime::now(),
            };
            set_created(&file, created)?;
        }
        step("created")?;
        full_sync(&file)?;
        crate::flush_name(&dir, name);
        Ok(Shown::Shown(identity(&file)?.0))
    }
}
