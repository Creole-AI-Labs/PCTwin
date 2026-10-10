//! Undo's removal of a copy on Linux and macOS: move it aside first, then check (Security Design
//! 3B, undo bullet, decided 9 October 2026).
//!
//! These systems cannot remove a file through an open handle, so a name must be acted on. The
//! copy is opened and checked through a handle, then its name is moved, never replacing
//! anything, to a random private name in the same folder that the journal recorded first; what
//! moved is proven to be that very file, still unchanged, and only then removed. Anything else
//! found there goes back under its name, or beside it under a visible name; nothing is lost and
//! nothing is left hidden. After a crash, [`Destination::resolve_removing`] and its siblings
//! finish every recorded private name the same way.
//!
//! The steps (numbers as in the design):
//! 1. [`Destination::check_copy`], no permit: open relative to the folder without following
//!    links (`O_NOFOLLOW|O_NONBLOCK|O_NOCTTY`); a regular file, on a drive on the allow-list,
//!    not immutable or append-only, the recorded identity (with its birth time), one name; no
//!    database working files or lock files beside it.
//! 2. `verify` through the handle (size, modified time, fingerprint).
//! 3. A change in the last 10 ms is waited out and looked at again.
//! 4. The caller takes an undo permit and records `Removing { private }` durably (in batches).
//! 5. Linux: a write lease on the handle (nobody else has it open), or the person's batch
//!    confirmation where the system cannot say. macOS: file coordination for deleting (apps
//!    showing the file save and let go; steps 6 to 9 run while they wait, and a coordination that
//!    does not answer in time removes nothing), then the system's list of programs with the file
//!    open; if either cannot be asked, nothing is removed.
//! 6. The name moves to the private name, never replacing.
//! 7. What is under the private name is proven to be the held file, unchanged; else it goes back.
//! 8. Right before the removal it is proven again, in every way, change time included
//!    ([`finish`], the one way a private name is ever removed, after a crash too); then removed.
//! 9. The held file is checked once more: no names left and unchanged is removed; anything
//!    written meanwhile is saved beside it under a visible name.
//!
//! Every rename, removal and new name goes through [`Acts`], which refuses it once the undo right
//! it runs under has ended (a resolution that ran out of time): a late worker never acts.

use std::io;

use rustix::fs::{AtFlags, FileType, Mode, OFlags, RenameFlags};
use rustix::io::Errno;

#[cfg(target_os = "macos")]
use crate::macos_check;
use crate::siblings::{MOST_TRIES, sibling_name};
use crate::{Destination, Dir, FileId, GateError, Removed, birth_hold, identity, sync_folder};

impl From<Errno> for GateError {
    fn from(e: Errno) -> Self {
        GateError::Io(e.into())
    }
}

/// What a removal records in the journal before each step that moves or makes a name, so a
/// crash at any point leaves only names the journal knows. The caller writes it durably and
/// returns only once it is on disk; an error stops the removal before the step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step<'a> {
    /// The private name recorded for this removal was taken by something else: this new one is
    /// used instead.
    NewPrivate { private: &'a str },
    /// About to move whatever is under `private` to `to` (its own name, or a visible name).
    Putting { private: &'a str, to: &'a str },
    /// About to save bytes into the new private file `temp`, to be named `to`. `complete` is
    /// false before the copy (the file is made with no permissions at all) and true once every
    /// byte is in it, flushed, with the file's permissions: only then is it named.
    Salvaging {
        temp: &'a str,
        to: &'a str,
        complete: bool,
    },
}

/// What the person's app passes in for one batch of removals.
pub struct Context<'a> {
    /// The catalog words for a file kept beside its own name, in the person's language
    /// (" (kept by PCTwin undo)").
    pub kept_words: &'a str,
    /// The byte length of the longest of those words in any language.
    pub room_for_words: usize,
    /// The person confirmed, once for this batch, that their other programs are closed: used
    /// only where the system cannot say whether another program has a file open.
    pub others_closed: bool,
    /// Records a step durably before it happens.
    pub journal: &'a mut dyn FnMut(Step<'_>) -> io::Result<()>,
}

/// A copy opened and checked (steps 1 to 3), held open, ready for [`Destination::remove_checked`].
pub struct Checked {
    dir: Dir,
    folder: String,
    name: String,
    handle: std::fs::File,
    look0: Look,
    file: FileId,
    dir_id: FileId,
}

impl Checked {
    /// The copy's identity.
    pub fn file(&self) -> FileId {
        self.file
    }

    /// Its folder's identity, recorded with the private name so the folder can be found again
    /// if it is moved after a crash.
    pub fn dir_id(&self) -> FileId {
        self.dir_id
    }
}

/// How checking a copy ended.
pub enum Check {
    /// Checked; nothing has been changed.
    Ready(Box<Checked>),
    /// Finished without changing anything.
    Done(Removed),
}

/// A new private name for a copy's way out: `.pctwin-undo-` and 128 random bits from the system.
pub fn private_name() -> io::Result<String> {
    let mut bits = [0u8; 16];
    getrandom::fill(&mut bits).map_err(|e| io::Error::other(e.to_string()))?;
    Ok(format!(".pctwin-undo-{}", hex(&bits)))
}

/// A new private name for bytes being saved: `.pctwin-salvage-` and 128 random bits.
fn salvage_name() -> io::Result<String> {
    let mut bits = [0u8; 16];
    getrandom::fill(&mut bits).map_err(|e| io::Error::other(e.to_string()))?;
    Ok(format!(".pctwin-salvage-{}", hex(&bits)))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Whether `name` is one of undo's private names.
pub(crate) fn is_private_name(name: &str) -> bool {
    let token = |prefix: &str| {
        name.strip_prefix(prefix).is_some_and(|t| {
            t.len() == 32
                && t.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
    };
    token(".pctwin-undo-") || token(".pctwin-salvage-")
}

/// Every rename, removal and new name undo makes here goes through this, and each is refused once
/// the undo right it runs under is no longer in force: a resolution that ran out of time and was
/// given up on never acts later. (A step already inside the system when the right ends finishes;
/// every such step was journaled before it began, so the journal still explains the disk.)
struct Acts<'a> {
    in_force: &'a dyn Fn() -> bool,
}

impl Acts<'_> {
    /// An error once the right has ended.
    fn go(&self) -> Result<(), GateError> {
        if (self.in_force)() {
            Ok(())
        } else {
            Err(GateError::Io(io::Error::new(
                io::ErrorKind::TimedOut,
                "undo stopped here: it took too long",
            )))
        }
    }

    /// Moves `from` to `to` in `dir`, never replacing anything.
    fn rename(&self, dir: &Dir, from: &str, to: &str) -> Result<(), Errno> {
        if !(self.in_force)() {
            return Err(Errno::CANCELED);
        }
        rustix::fs::renameat_with(dir, from, dir, to, RenameFlags::NOREPLACE)
    }

    /// Removes the name `name` in `dir` (only ever from [`finish`]).
    fn unlink(&self, dir: &Dir, name: &str) -> Result<(), Errno> {
        if !(self.in_force)() {
            return Err(Errno::CANCELED);
        }
        rustix::fs::unlinkat(dir, name, AtFlags::empty())
    }

    /// Makes the new file `name` in `dir`, with no permissions at all, open for writing.
    fn create(&self, dir: &Dir, name: &str) -> Result<std::fs::File, Errno> {
        if !(self.in_force)() {
            return Err(Errno::CANCELED);
        }
        rustix::fs::openat(
            dir,
            name,
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map(std::fs::File::from)
    }
}

/// What tells a held file changed: size, modified time, change time (which a program cannot set
/// back), and its count of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Look {
    len: u64,
    modified: (i64, i64),
    changed: (i64, i64),
    names: u64,
}

impl Look {
    fn of(file: &impl std::os::fd::AsFd) -> io::Result<Self> {
        let st = rustix::fs::fstat(file)?;
        Ok(Self::from(&st))
    }

    fn from(st: &rustix::fs::Stat) -> Self {
        #[allow(
            clippy::unnecessary_cast,
            reason = "field types differ between systems"
        )]
        Self {
            len: st.st_size as u64,
            modified: (st.st_mtime as i64, st.st_mtime_nsec as i64),
            changed: (st.st_ctime as i64, st.st_ctime_nsec as i64),
            names: st.st_nlink as u64,
        }
    }

    /// The same contents as `other` as far as size and modified time tell (the change time moves
    /// with PCTwin's own rename and removal, so it is not compared across those).
    fn same_contents(&self, other: &Look) -> bool {
        self.len == other.len && self.modified == other.modified
    }
}

/// Whether two looks are at the very same file (same drive, same number).
fn same_file(a: &rustix::fs::Stat, b: &rustix::fs::Stat) -> bool {
    a.st_dev == b.st_dev && a.st_ino == b.st_ino
}

/// Whether the drive under `file` is one undo removes from: it must keep birth times and
/// tell files apart reliably (Linux ext4, XFS, btrfs, ZFS, F2FS; macOS APFS, writable).
fn drive_allowed(file: &std::fs::File) -> io::Result<bool> {
    #[cfg(target_os = "linux")]
    {
        const EXT4: u32 = 0xEF53;
        const XFS: u32 = 0x5846_5342;
        const BTRFS: u32 = 0x9123_683E;
        const ZFS: u32 = 0x2FC1_2FC1;
        const F2FS: u32 = 0xF2F5_2010;
        let st = rustix::fs::fstatfs(file)?;
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "the drive types are 32-bit numbers in a wider field"
        )]
        let kind = st.f_type as u32;
        Ok(matches!(kind, EXT4 | XFS | BTRFS | ZFS | F2FS))
    }
    #[cfg(target_os = "macos")]
    {
        const MNT_RDONLY: u32 = 0x0000_0001;
        let st = rustix::fs::fstatfs(file)?;
        let name: Vec<u8> = st
            .f_fstypename
            .iter()
            .take_while(|c| **c != 0)
            .map(|c| c.to_ne_bytes()[0])
            .collect();
        Ok(name == b"apfs" && st.f_flags & MNT_RDONLY == 0)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = file;
        Ok(false)
    }
}

/// Whether the file is marked unchangeable or add-only (it then cannot be moved or removed).
fn locked_by_flags(file: &std::fs::File) -> bool {
    #[cfg(target_os = "linux")]
    {
        use rustix::fs::IFlags;
        rustix::fs::ioctl_getflags(file)
            .is_ok_and(|f| f.intersects(IFlags::IMMUTABLE | IFlags::APPEND))
    }
    #[cfg(target_os = "macos")]
    {
        const LOCKED: u32 = 0x0000_0002 | 0x0000_0004 | 0x0002_0000 | 0x0004_0000;
        rustix::fs::fstat(file).is_ok_and(|st| st.st_flags & LOCKED != 0)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = file;
        false
    }
}

/// The names of an app's working files that sit beside a file it has open (a database's journal
/// and lock files, an office program's owner file).
fn working_files(name: &str) -> [String; 7] {
    [
        format!("{name}-wal"),
        format!("{name}-shm"),
        format!("{name}-journal"),
        format!("{name}.lock"),
        format!("~${name}"),
        format!(".~lock.{name}#"),
        format!("{name}.lck"),
    ]
}

/// File types an app keeps open while it runs (mail stores, password safes, databases).
fn is_store_type(name: &str, head: &[u8]) -> bool {
    let ext = name
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    matches!(
        ext.as_str(),
        "sqlite" | "sqlite3" | "pst" | "ost" | "mbox" | "kdbx"
    ) || head.starts_with(b"SQLite format 3\0")
}

impl Destination {
    /// Steps 1 to 3 for the copy at the stored path `stored`, recorded as `expect`, with the
    /// caller's check `verify` (given the held file). Nothing is changed and no permit is
    /// needed; a copy that passes is held open for [`remove_checked`](Self::remove_checked).
    pub fn check_copy(
        &self,
        stored: &str,
        expect: FileId,
        verify: impl FnOnce(&mut std::fs::File) -> io::Result<bool>,
    ) -> Result<Check, GateError> {
        let done = |r| Ok(Check::Done(r));
        if expect.born.is_none() {
            return done(Removed::Unsupported);
        }
        birth_hold::settle();
        let Some((dir, name)) = self.open_stored_folder(stored)? else {
            return done(Removed::Gone);
        };
        if is_private_name(name) || crate::is_temp_name(name) || crate::is_undo_name(name) {
            return Err(GateError::Io(crate::invalid("not a copy undo removes")));
        }
        let dir_id = identity(&dir.try_clone()?.into_std_file())?.0;
        let mut handle = match rustix::fs::openat(
            &dir,
            name,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::NOCTTY | OFlags::CLOEXEC,
            Mode::empty(),
        ) {
            Ok(fd) => std::fs::File::from(fd),
            Err(Errno::NOENT) => return done(Removed::Gone),
            // A link at the name (never followed), or something that is not a file.
            Err(Errno::LOOP | Errno::NXIO | Errno::ISDIR) => return done(Removed::NotThatFile),
            Err(e) => return Err(GateError::Io(e.into())),
        };
        let st = rustix::fs::fstat(&handle)?;
        if FileType::from_raw_mode(st.st_mode) != FileType::RegularFile {
            return done(Removed::NotThatFile);
        }
        if !drive_allowed(&handle)? {
            return done(Removed::Unsupported);
        }
        let (id, names) = identity(&handle)?;
        if id != expect {
            return done(Removed::NotThatFile);
        }
        if let Some(kept) = crate::by_names(names) {
            return done(kept);
        }
        if locked_by_flags(&handle) {
            return done(Removed::Unsupported);
        }
        let mut head = [0u8; 16];
        let got = read_at(&handle, &mut head, 0)?;
        if is_store_type(name, &head[..got])
            || working_files(name)
                .iter()
                .any(|w| rustix::fs::statat(&dir, w.as_str(), AtFlags::SYMLINK_NOFOLLOW).is_ok())
        {
            return done(Removed::AppKeepsOpen);
        }
        let look0 = Look::from(&st);
        if !verify(&mut handle)? || Look::of(&handle)? != look0 {
            return done(Removed::Changed);
        }
        // A change in the last 10 ms may not show yet in the drive's clock ticks: wait it out
        // and look again.
        if changed_lately(&look0) {
            std::thread::sleep(birth_hold::HOLD);
            if Look::of(&handle)? != look0 {
                return done(Removed::Changed);
            }
        }
        let folder = stored.rsplit_once('/').map_or("", |(f, _)| f).to_string();
        Ok(Check::Ready(Box::new(Checked {
            dir,
            folder,
            name: name.to_string(),
            handle,
            look0,
            file: id,
            dir_id,
        })))
    }

    /// Steps 5 to 9 for a checked copy, whose removal through `private` the journal holds. Takes
    /// an undo right, so nothing can be removed once undo is closed for good (and nothing once a
    /// lent right has ended). Whatever it changed is flushed to the disk through the very folder
    /// it was checked in before it returns, so the caller may record the outcome at once.
    pub fn remove_checked(
        &self,
        right: &impl pctwin_journal::UndoRight,
        copy: Checked,
        private: &str,
        cx: &mut Context<'_>,
    ) -> Result<Removed, GateError> {
        let Checked {
            dir,
            folder,
            name,
            handle,
            look0,
            file: _,
            dir_id: _,
        } = copy;
        let in_force = || right.in_force();
        let acts = Acts {
            in_force: &in_force,
        };
        acts.go()?;
        // 5. Nobody else has it open.
        #[cfg(target_os = "macos")]
        let r = {
            let Some(path) = self.root_path.as_ref().map(|r| r.join(&folder).join(&name)) else {
                return Ok(Removed::CannotCheck);
            };
            // Apps presenting the file are told and let go; the rest happens while they wait. A
            // coordination that does not answer in time, or fails, removes nothing.
            let done = macos_check::coordinated_for_deleting(&path, || {
                match macos_check::others_have_it_open(&path) {
                    Ok(true) => Ok(Removed::InUse),
                    Err(_) => Ok(Removed::CannotCheck),
                    Ok(false) => move_prove_remove(
                        &dir, &folder, &name, &handle, look0, private, false, &acts, cx,
                    ),
                }
            });
            match done {
                Ok(Some(r)) => r,
                Ok(None) | Err(_) => Ok(Removed::CannotCheck),
            }
        };
        #[cfg(not(target_os = "macos"))]
        let r = {
            let leased = match others_have_it_open(&handle, cx.others_closed) {
                Openers::None { leased } => leased,
                Openers::Some => return Ok(Removed::InUse),
                Openers::CannotCheck => return Ok(Removed::CannotCheck),
            };
            let r = move_prove_remove(
                &dir, &folder, &name, &handle, look0, private, leased, &acts, cx,
            );
            #[cfg(target_os = "linux")]
            if leased {
                let _ = pctwin_lease::release(&handle);
            }
            r
        };
        let r = r?;
        // On the disk before anyone records it, through the folder it happened in.
        sync_folder(&dir)?;
        Ok(r)
    }
}

/// Steps 6 to 9 (see [`Destination::remove_checked`]); `leased` says a Linux write lease is held
/// on `handle` (the caller gives it up afterwards).
#[expect(clippy::too_many_arguments, reason = "the parts of one checked copy")]
fn move_prove_remove(
    dir: &Dir,
    folder: &str,
    name: &str,
    handle: &std::fs::File,
    look0: Look,
    private: &str,
    leased: bool,
    acts: &Acts<'_>,
    cx: &mut Context<'_>,
) -> Result<Removed, GateError> {
    let at = |n: &str| crate::stored_path(folder, n);
    if Look::of(handle)? != look0 {
        return Ok(Removed::Changed);
    }
    // 6. Moved aside, never replacing.
    let mut private = private.to_string();
    let mut tries = 0;
    loop {
        match acts.rename(dir, name, private.as_str()) {
            Ok(()) => break,
            Err(Errno::NOENT) => return Ok(Removed::Gone),
            Err(Errno::EXIST) if tries < 8 => {
                tries += 1;
                private = private_name()?;
                (cx.journal)(Step::NewPrivate { private: &private })?;
            }
            Err(Errno::INVAL | Errno::NOTSUP | Errno::NOSYS | Errno::PERM) => {
                return Ok(Removed::Unsupported);
            }
            Err(e) => return Err(e.into()),
        }
    }
    // 7. What moved is the held file, unchanged, with one name; anything else goes back.
    let held = rustix::fs::fstat(handle)?;
    let ours = match rustix::fs::statat(dir, private.as_str(), AtFlags::SYMLINK_NOFOLLOW) {
        Ok(moved) => same_file(&moved, &held),
        Err(Errno::NOENT) => false,
        Err(e) => return Err(e.into()),
    };
    let now = Look::from(&held);
    if !ours || now.names != 1 || !now.same_contents(&look0) {
        return Ok(match put_back(dir, &private, name, acts, cx)? {
            Back::Home if !ours => Removed::NotThatFile,
            Back::Home => Removed::Changed,
            Back::Beside(n) => Removed::KeptBeside { at: at(&n) },
        });
    }
    // 8 and 9. `now` was looked at after PCTwin's own rename: from here nothing of PCTwin's
    // touches the file before the removal, so every part of it must still match.
    Ok(
        match finish(dir, handle, name, &private, now, leased, acts, cx)? {
            Finish::Removed => Removed::Removed,
            Finish::Saved(to) => Removed::Salvaged { at: at(&to) },
            Finish::NotProven(Unproven::Gone) => Removed::Gone,
            Finish::NotProven(why) => match put_back(dir, &private, name, acts, cx)? {
                Back::Home => match why {
                    Unproven::OpenedMeanwhile => Removed::InUse,
                    Unproven::NotThatFile => Removed::NotThatFile,
                    Unproven::Changed | Unproven::Gone => Removed::Changed,
                },
                Back::Beside(n) => Removed::KeptBeside { at: at(&n) },
            },
        },
    )
}

/// How removing a copy proven under its private name ended.
#[derive(Debug, PartialEq, Eq)]
enum Finish {
    /// Removed; nothing was written to it meanwhile.
    Removed,
    /// Removed, and what was written to it meanwhile is saved whole under this visible name.
    Saved(String),
    /// Not removed: right before the removal it could not be proven (why). Nothing was changed.
    NotProven(Unproven),
}

/// Why a copy could not be proven right before its removal.
#[derive(Debug, PartialEq, Eq)]
enum Unproven {
    /// Someone started to open it (Linux: the lease is no longer held).
    OpenedMeanwhile,
    /// It no longer looks exactly as it did (size, modified time, change time or names).
    Changed,
    /// Something else is under the private name now.
    NotThatFile,
    /// Nothing is under the private name now (moved away by someone else).
    Gone,
}

/// Steps 8 and 9: the one way a private name is ever removed, on the way through and after a
/// crash alike. Right before the removal it proves, in this order: nobody has started to open
/// the held file (Linux, `leased`); the held file still looks exactly as `expect` in every way,
/// its change time included (`expect` was taken after PCTwin's last rename of it); and what is
/// under `private` now is that very file. Only then is the name removed. Then the held file is
/// looked at once more: no names left and unchanged is done; bytes written meanwhile (or a
/// program that started to open it) are saved beside it under a visible name.
#[expect(clippy::too_many_arguments, reason = "the parts of one proven copy")]
fn finish(
    dir: &Dir,
    held: &std::fs::File,
    name: &str,
    private: &str,
    expect: Look,
    leased: bool,
    acts: &Acts<'_>,
    cx: &mut Context<'_>,
) -> Result<Finish, GateError> {
    if leased && !lease_still_held(held) {
        return Ok(Finish::NotProven(Unproven::OpenedMeanwhile));
    }
    let held_now = rustix::fs::fstat(held)?;
    if Look::from(&held_now) != expect {
        return Ok(Finish::NotProven(Unproven::Changed));
    }
    match rustix::fs::statat(dir, private, AtFlags::SYMLINK_NOFOLLOW) {
        Ok(under) if same_file(&under, &held_now) => {}
        Ok(_) => return Ok(Finish::NotProven(Unproven::NotThatFile)),
        Err(Errno::NOENT) => return Ok(Finish::NotProven(Unproven::Gone)),
        Err(e) => return Err(e.into()),
    }
    match acts.unlink(dir, private) {
        Ok(()) | Err(Errno::NOENT) => {}
        Err(e) => return Err(e.into()),
    }
    let after = Look::of(held)?;
    let opened_meanwhile = leased && !lease_still_held(held);
    if after.names == 0 && after.same_contents(&expect) && !opened_meanwhile {
        return Ok(Finish::Removed);
    }
    // A program that started to open it at the last moment expects it under its own name, so a
    // copy goes back there if that is free.
    let first = opened_meanwhile.then_some(name);
    Ok(Finish::Saved(salvage(dir, held, name, first, acts, cx)?))
}

/// Where a file that went back ended.
enum Back {
    /// Under its own name.
    Home,
    /// Under this visible name beside it.
    Beside(String),
}

/// Moves whatever is under `private` back to `name`, never replacing; if `name` was taken, to
/// the first free visible name beside it. Each target is recorded first.
fn put_back(
    dir: &Dir,
    private: &str,
    name: &str,
    acts: &Acts<'_>,
    cx: &mut Context<'_>,
) -> Result<Back, GateError> {
    (cx.journal)(Step::Putting { private, to: name })?;
    match acts.rename(dir, private, name) {
        Ok(()) => return Ok(Back::Home),
        Err(Errno::EXIST) => {}
        Err(e) => return Err(GateError::Io(e.into())),
    }
    for n in 1..=MOST_TRIES {
        let to = sibling_name(name, cx.kept_words, n, cx.room_for_words);
        (cx.journal)(Step::Putting { private, to: &to })?;
        match acts.rename(dir, private, to.as_str()) {
            Ok(()) => return Ok(Back::Beside(to)),
            Err(Errno::EXIST) => continue,
            Err(e) => return Err(GateError::Io(e.into())),
        }
    }
    Err(GateError::TooManyClashes)
}

/// Saves the held file's bytes, as they are now, under a new name beside `name` (or `name`
/// itself first, if `first` says so and it is free), keeping its permissions, modified time and
/// extended attributes. Never touches a file of the person's: the bytes go into a new private
/// file, made with no permissions at all; only once every byte is in it and flushed does it get
/// the file's permissions, and is it recorded complete, and only then is it named, without
/// replacing anything. If PCTwin stops before that, the journal says it may be incomplete.
fn salvage(
    dir: &Dir,
    held: &std::fs::File,
    name: &str,
    first: Option<&str>,
    acts: &Acts<'_>,
    cx: &mut Context<'_>,
) -> Result<String, GateError> {
    let temp = salvage_name()?;
    let mut targets: Vec<String> = first.map(str::to_string).into_iter().collect();
    targets
        .extend((1..=MOST_TRIES).map(|n| sibling_name(name, cx.kept_words, n, cx.room_for_words)));
    (cx.journal)(Step::Salvaging {
        temp: &temp,
        to: &targets[0],
        complete: false,
    })?;
    let mut out = acts.create(dir, temp.as_str())?;
    copy_from(held, &mut out, acts)?;
    let st = rustix::fs::fstat(held)?;
    copy_attributes(held, &out);
    #[allow(
        clippy::unnecessary_cast,
        reason = "field types differ between systems"
    )]
    let modified = rustix::fs::Timespec {
        tv_sec: st.st_mtime as _,
        tv_nsec: st.st_mtime_nsec as _,
    };
    rustix::fs::futimens(
        &out,
        &rustix::fs::Timestamps {
            last_access: modified,
            last_modification: modified,
        },
    )?;
    // Every byte is in: only now does it get the file's permissions.
    rustix::fs::fchmod(
        &out,
        Mode::from_raw_mode(st.st_mode) & Mode::from_bits_truncate(0o7777),
    )?;
    out.sync_all()?;
    // Recorded complete (with each name tried), and only then named.
    for to in &targets {
        (cx.journal)(Step::Salvaging {
            temp: &temp,
            to,
            complete: true,
        })?;
        match acts.rename(dir, temp.as_str(), to.as_str()) {
            Ok(()) => return Ok(to.clone()),
            Err(Errno::EXIST) => continue,
            Err(e) => return Err(GateError::Io(e.into())),
        }
    }
    Err(GateError::TooManyClashes)
}

/// Copies every byte of `from` (read from its start, by position) into `to`, stopping if the
/// undo right ends meanwhile.
fn copy_from(
    from: &std::fs::File,
    to: &mut std::fs::File,
    acts: &Acts<'_>,
) -> Result<(), GateError> {
    use std::io::Write;
    let mut buf = vec![0u8; 1 << 16];
    let mut at = 0u64;
    loop {
        acts.go()?;
        let n = read_at(from, &mut buf, at)?;
        if n == 0 {
            return Ok(());
        }
        to.write_all(&buf[..n])?;
        at += n as u64;
    }
}

fn read_at(file: &std::fs::File, buf: &mut [u8], at: u64) -> io::Result<usize> {
    use std::os::unix::fs::FileExt;
    loop {
        match file.read_at(buf, at) {
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            other => return other,
        }
    }
}

/// Copies extended attributes, best effort (a drive or attribute that refuses is skipped: the
/// bytes, permissions and time matter most).
fn copy_attributes(from: &std::fs::File, to: &std::fs::File) {
    let mut names = vec![0u8; 64 * 1024];
    let Ok(n) = rustix::fs::flistxattr(from, &mut names[..]) else {
        return;
    };
    let mut value = vec![0u8; 64 * 1024];
    for attr in names[..n].split(|b| *b == 0).filter(|a| !a.is_empty()) {
        let Ok(attr) = std::ffi::CString::new(attr) else {
            continue;
        };
        if let Ok(len) = rustix::fs::fgetxattr(from, attr.as_c_str(), &mut value[..]) {
            let _ = rustix::fs::fsetxattr(
                to,
                attr.as_c_str(),
                &value[..len],
                rustix::fs::XattrFlags::empty(),
            );
        }
    }
}

/// Gives the file under `name` in `dir` read and write for its owner, through the very file
/// (never by following a link): a salvage cut short still has no permissions at all.
fn make_readable(dir: &Dir, name: &str) -> Result<(), GateError> {
    #[cfg(target_os = "linux")]
    {
        use std::os::fd::AsRawFd;
        // A handle that only names the file (no reading needed, so no permission needed), then
        // the change through the system's own link to that handle: the very file opened.
        let fd = rustix::fs::openat(
            dir,
            name,
            OFlags::PATH | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )?;
        let st = rustix::fs::fstat(&fd)?;
        if FileType::from_raw_mode(st.st_mode) != FileType::RegularFile {
            return Err(GateError::Io(crate::invalid("not a file PCTwin saved")));
        }
        rustix::fs::chmod(
            format!("/proc/self/fd/{}", fd.as_raw_fd()).as_str(),
            Mode::RUSR | Mode::WUSR,
        )?;
        Ok(())
    }
    #[cfg(not(target_os = "linux"))]
    {
        // macOS changes a link itself, never what it points to, when told not to follow.
        rustix::fs::chmodat(
            dir,
            name,
            Mode::RUSR | Mode::WUSR,
            AtFlags::SYMLINK_NOFOLLOW,
        )?;
        Ok(())
    }
}

/// A change within the last 10 ms (the drive's clock ticks may hide a second one).
fn changed_lately(look: &Look) -> bool {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| {
            (
                i64::try_from(d.as_secs()).unwrap_or(i64::MAX),
                i64::from(d.subsec_nanos()),
            )
        })
        .unwrap_or((0, 0));
    let ns = |(s, n): (i64, i64)| i128::from(s) * 1_000_000_000 + i128::from(n);
    ns(now) - ns(look.changed) < 10_000_000
}

/// Whether anyone else has the held file open.
enum Openers {
    None { leased: bool },
    Some,
    CannotCheck,
}

#[cfg(not(target_os = "macos"))]
fn others_have_it_open(handle: &std::fs::File, others_closed: bool) -> Openers {
    #[cfg(target_os = "linux")]
    {
        use pctwin_lease::Lease;
        // A program PCTwin starts at this moment briefly shares every handle, which reads as
        // "open elsewhere": asked again twice before believing it.
        for wait in [0u64, 20, 50] {
            std::thread::sleep(std::time::Duration::from_millis(wait));
            match pctwin_lease::write_lease(handle) {
                Lease::Held => return Openers::None { leased: true },
                Lease::InUse => continue,
                Lease::CannotCheck(_) if others_closed => return Openers::None { leased: false },
                Lease::CannotCheck(_) => return Openers::CannotCheck,
            }
        }
        Openers::Some
    }
    #[cfg(not(target_os = "linux"))]
    {
        // Other Unix systems: nothing can be asked; only the person's confirmation lets the
        // removal go ahead.
        let _ = handle;
        if others_closed {
            Openers::None { leased: false }
        } else {
            Openers::CannotCheck
        }
    }
}

fn lease_still_held(handle: &std::fs::File) -> bool {
    #[cfg(target_os = "linux")]
    {
        pctwin_lease::still_held(handle)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = handle;
        true
    }
}

/// How finishing a removal the journal recorded part of the way ended (after a crash or a stop,
/// at the next start and again when undo closes). Every answer is decided from the disk, is
/// flushed to the disk before it is given, and is safe to reach again: running a resolution
/// twice ends the same way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    /// The very file, unchanged, was under its private name and is now removed.
    Removed,
    /// Nothing of it is left under its name or its private name: it was already removed (or
    /// moved away by the person).
    AlreadyGone,
    /// The copy itself is under its own name and nothing is under the private name (undo stopped
    /// before moving it, or it was put back): it is to be looked at again as usual, never called
    /// changed for having been put back.
    StillThere,
    /// Put back under its own name: something that is not PCTwin's copy, or the copy changed
    /// since (never the copy itself merely put back: that is [`Resolution::StillThere`] next
    /// time it is looked at).
    Home,
    /// Put back under its own name: another program has it open (try again later).
    InUse,
    /// Put back under its own name: the system cannot say whether another program has it open,
    /// and the person has not confirmed their other programs are closed.
    CannotCheck,
    /// Kept under the visible name `at` beside its own (the stored path).
    KeptAt { at: String },
    /// What a program wrote to the copy while undo removed it, saved whole under the visible
    /// name `at` (the stored path).
    SavedAgain { at: String },
    /// What a program wrote to the copy while undo removed it, saved under the visible name `at`,
    /// but PCTwin stopped while copying it: it may be incomplete. Readable by its owner.
    SavedMaybeIncomplete { at: String },
    /// Its folder cannot be found on this drive (unplugged, or moved somewhere undo cannot find
    /// it): not resolved, nothing changed.
    FolderMissing,
}

/// How many folders the search for a folder moved after a crash looks in, at most.
const MOST_FOLDERS_SEARCHED: usize = 20_000;

/// What is under a name, looked at without following a link.
enum Under {
    /// Nothing.
    Nothing,
    /// A regular file, opened, and proven to be the one the name showed.
    File(std::fs::File),
    /// Something else: a link, a folder, a pipe, a device, or a file PCTwin may not read. Never
    /// PCTwin's copy, which is a regular file PCTwin read.
    Other,
}

/// Looks at what is under `name` in `dir` (never following a link, never waiting on a pipe). A
/// look the drive refuses is an error, never "nothing".
fn look_under(dir: &Dir, name: &str) -> Result<Under, GateError> {
    for _ in 0..3 {
        let seen = match rustix::fs::statat(dir, name, AtFlags::SYMLINK_NOFOLLOW) {
            Ok(st) => st,
            Err(Errno::NOENT) => return Ok(Under::Nothing),
            Err(e) => return Err(e.into()),
        };
        if FileType::from_raw_mode(seen.st_mode) != FileType::RegularFile {
            return Ok(Under::Other);
        }
        let file = match rustix::fs::openat(
            dir,
            name,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::NOCTTY | OFlags::CLOEXEC,
            Mode::empty(),
        ) {
            Ok(fd) => std::fs::File::from(fd),
            Err(Errno::ACCESS | Errno::PERM) => return Ok(Under::Other),
            // Swapped between the look and the open: look again.
            Err(Errno::NOENT | Errno::LOOP | Errno::NXIO | Errno::ISDIR) => continue,
            Err(e) => return Err(e.into()),
        };
        let opened = rustix::fs::fstat(&file)?;
        if same_file(&opened, &seen)
            && FileType::from_raw_mode(opened.st_mode) == FileType::RegularFile
        {
            return Ok(Under::File(file));
        }
    }
    Err(GateError::Io(io::Error::other(
        "what is under this name keeps changing",
    )))
}

/// Whether anything at all is under `name` in `dir` (never following a link). A look the drive
/// refuses is an error.
fn present(dir: &Dir, name: &str) -> Result<bool, GateError> {
    match rustix::fs::statat(dir, name, AtFlags::SYMLINK_NOFOLLOW) {
        Ok(_) => Ok(true),
        Err(Errno::NOENT) => Ok(false),
        Err(e) => Err(e.into()),
    }
}

/// Whether the copy `file` itself is under `name` in `dir`.
fn holds_copy(dir: &Dir, name: &str, file: FileId) -> Result<bool, GateError> {
    Ok(match look_under(dir, name)? {
        Under::File(f) => identity(&f)?.0 == file,
        Under::Nothing | Under::Other => false,
    })
}

impl Destination {
    /// Finishes a removal recorded as `Removing { file, dir_id, private }` for the copy at the
    /// stored path `stored`, with the caller's check `verify` for "the very file, unchanged".
    #[expect(
        clippy::too_many_arguments,
        reason = "exactly the journal's record and the check"
    )]
    pub fn resolve_removing(
        &self,
        right: &impl pctwin_journal::UndoRight,
        stored: &str,
        file: FileId,
        dir_id: FileId,
        private: &str,
        verify: impl FnOnce(&mut std::fs::File) -> io::Result<bool>,
        cx: &mut Context<'_>,
    ) -> Result<Resolution, GateError> {
        if !is_private_name(private) {
            return Err(GateError::Io(crate::invalid(
                "not one of undo's private names",
            )));
        }
        let in_force = || right.in_force();
        let acts = Acts {
            in_force: &in_force,
        };
        acts.go()?;
        let name = stored.rsplit('/').next().unwrap_or(stored);
        let Some((dir, folder)) = self.locate(stored, dir_id, private)? else {
            return Ok(Resolution::FolderMissing);
        };
        let r = match look_under(&dir, private)? {
            // Nothing under the private name: it was never moved, or it is already removed.
            Under::Nothing => {
                if holds_copy(&dir, name, file)? {
                    Resolution::StillThere
                } else {
                    Resolution::AlreadyGone
                }
            }
            // Not PCTwin's copy (a link, a folder, a pipe...): back under the name, visibly.
            Under::Other => put_back_as(&dir, &folder, private, name, Resolution::Home, &acts, cx)?,
            Under::File(held) => {
                self.finish_removing(&dir, &folder, name, private, held, file, verify, &acts, cx)?
            }
        };
        // What happened here, or in an earlier try that stopped before its flush, is on the disk
        // before anyone records it.
        sync_folder(&dir)?;
        Ok(r)
    }

    /// The private name holds a regular file: removed if it is the copy, unchanged, and nobody
    /// else has it open; otherwise it goes back.
    #[expect(
        clippy::too_many_arguments,
        reason = "the record, the file and the check"
    )]
    fn finish_removing(
        &self,
        dir: &Dir,
        folder: &str,
        name: &str,
        private: &str,
        mut held: std::fs::File,
        file: FileId,
        verify: impl FnOnce(&mut std::fs::File) -> io::Result<bool>,
        acts: &Acts<'_>,
        cx: &mut Context<'_>,
    ) -> Result<Resolution, GateError> {
        let at = |n: &str| crate::stored_path(folder, n);
        let before = Look::of(&held)?;
        let (id, names) = identity(&held)?;
        let mut kept = Resolution::Home;
        if id == file && names == 1 && verify(&mut held)? && Look::of(&held)? == before {
            // A program that had it open before PCTwin stopped may still be writing to it: such
            // a file goes back under its name for another try, as at any other time.
            match self.nobody_else_has(&held, folder, private, cx.others_closed) {
                Openers::None { leased } => {
                    let r = finish(dir, &held, name, private, before, leased, acts, cx);
                    #[cfg(target_os = "linux")]
                    if leased {
                        let _ = pctwin_lease::release(&held);
                    }
                    match r? {
                        Finish::Removed => return Ok(Resolution::Removed),
                        Finish::Saved(to) => return Ok(Resolution::SavedAgain { at: at(&to) }),
                        Finish::NotProven(Unproven::Gone) => return Ok(Resolution::AlreadyGone),
                        Finish::NotProven(Unproven::OpenedMeanwhile) => kept = Resolution::InUse,
                        Finish::NotProven(Unproven::Changed | Unproven::NotThatFile) => {}
                    }
                }
                Openers::Some => kept = Resolution::InUse,
                Openers::CannotCheck => kept = Resolution::CannotCheck,
            }
        }
        drop(held);
        put_back_as(dir, folder, private, name, kept, acts, cx)
    }

    /// Whether another program has the held file (under `private` in `folder`) open, for
    /// finishing a removal after PCTwin stopped.
    fn nobody_else_has(
        &self,
        held: &std::fs::File,
        folder: &str,
        private: &str,
        others_closed: bool,
    ) -> Openers {
        #[cfg(target_os = "macos")]
        {
            let _ = (held, others_closed);
            let Some(path) = self
                .root_path
                .as_ref()
                .map(|r| r.join(folder).join(private))
            else {
                return Openers::CannotCheck;
            };
            match macos_check::others_have_it_open(&path) {
                Ok(false) => Openers::None { leased: false },
                Ok(true) => Openers::Some,
                Err(_) => Openers::CannotCheck,
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (folder, private);
            others_have_it_open(held, others_closed)
        }
    }

    /// Finishes a recorded `Putting { private, to }` for the copy `file`: whatever is under
    /// `private` goes to `to` (or the next free visible name), or is confirmed already there. If
    /// it ends under the copy's own name and is the copy itself, it is
    /// [`Resolution::StillThere`], to be looked at again; never called changed for that.
    #[expect(
        clippy::too_many_arguments,
        reason = "exactly the journal's record and the copy"
    )]
    pub fn resolve_putting(
        &self,
        right: &impl pctwin_journal::UndoRight,
        stored: &str,
        file: FileId,
        dir_id: FileId,
        private: &str,
        to: &str,
        cx: &mut Context<'_>,
    ) -> Result<Resolution, GateError> {
        if !is_private_name(private) {
            return Err(GateError::Io(crate::invalid(
                "not one of undo's private names",
            )));
        }
        let in_force = || right.in_force();
        let acts = Acts {
            in_force: &in_force,
        };
        acts.go()?;
        let name = stored.rsplit('/').next().unwrap_or(stored);
        let Some((dir, folder)) = self.locate(stored, dir_id, private)? else {
            return Ok(Resolution::FolderMissing);
        };
        let ended = |to: &str| -> Result<Resolution, GateError> {
            if to != name {
                return Ok(Resolution::KeptAt {
                    at: crate::stored_path(&folder, to),
                });
            }
            Ok(if holds_copy(&dir, name, file)? {
                Resolution::StillThere
            } else {
                Resolution::Home
            })
        };
        let r = if present(&dir, private)? {
            match acts.rename(&dir, private, to) {
                Ok(()) => ended(to)?,
                Err(Errno::EXIST) => match put_back(&dir, private, name, &acts, cx)? {
                    Back::Home => ended(name)?,
                    Back::Beside(n) => ended(&n)?,
                },
                Err(e) => return Err(e.into()),
            }
        } else if present(&dir, to)? {
            // Already moved (the rename is all or nothing).
            ended(to)?
        } else {
            // Moved on since.
            Resolution::AlreadyGone
        };
        sync_folder(&dir)?;
        Ok(r)
    }

    /// Finishes a recorded `Salvaging { temp, to, complete }`: the saved bytes are given a
    /// visible name, never replacing anything. Unless the journal recorded them `complete`, they
    /// are published as "may be incomplete" (PCTwin stopped while copying), readable by their
    /// owner. Something that is not a regular file under `temp` is never PCTwin's: an error, so
    /// it is left exactly where it is and listed.
    #[expect(clippy::too_many_arguments, reason = "exactly the journal's record")]
    pub fn resolve_salvaging(
        &self,
        right: &impl pctwin_journal::UndoRight,
        stored: &str,
        dir_id: FileId,
        temp: &str,
        to: &str,
        complete: bool,
        cx: &mut Context<'_>,
    ) -> Result<Resolution, GateError> {
        if !is_private_name(temp) {
            return Err(GateError::Io(crate::invalid(
                "not one of undo's private names",
            )));
        }
        let in_force = || right.in_force();
        let acts = Acts {
            in_force: &in_force,
        };
        acts.go()?;
        let name = stored.rsplit('/').next().unwrap_or(stored);
        let Some((dir, folder)) = self.locate(stored, dir_id, temp)? else {
            return Ok(Resolution::FolderMissing);
        };
        let saved = |to: &str| {
            let at = crate::stored_path(&folder, to);
            if complete {
                Resolution::SavedAgain { at }
            } else {
                Resolution::SavedMaybeIncomplete { at }
            }
        };
        let r = match rustix::fs::statat(&dir, temp, AtFlags::SYMLINK_NOFOLLOW) {
            Err(Errno::NOENT) => {
                if present(&dir, to)? {
                    saved(to)
                } else {
                    Resolution::AlreadyGone
                }
            }
            Err(e) => return Err(e.into()),
            Ok(st) if FileType::from_raw_mode(st.st_mode) != FileType::RegularFile => {
                return Err(GateError::Io(crate::invalid(
                    "something that is not PCTwin's is under one of its private names",
                )));
            }
            Ok(st) => {
                // Cut short while copying, it still has no permissions at all.
                if Mode::from_raw_mode(st.st_mode) & Mode::from_bits_truncate(0o777)
                    == Mode::empty()
                {
                    acts.go()?;
                    make_readable(&dir, temp)?;
                }
                let mut targets = vec![to.to_string()];
                for n in 1..=MOST_TRIES {
                    let t = sibling_name(name, cx.kept_words, n, cx.room_for_words);
                    if t != to {
                        targets.push(t);
                    }
                }
                let mut named = None;
                for (i, target) in targets.iter().enumerate() {
                    if i > 0 {
                        (cx.journal)(Step::Salvaging {
                            temp,
                            to: target,
                            complete,
                        })?;
                    }
                    match acts.rename(&dir, temp, target.as_str()) {
                        Ok(()) => {
                            named = Some(target);
                            break;
                        }
                        Err(Errno::EXIST) => continue,
                        Err(e) => return Err(e.into()),
                    }
                }
                match named {
                    Some(target) => saved(target),
                    None => return Err(GateError::TooManyClashes),
                }
            }
        };
        sync_folder(&dir)?;
        Ok(r)
    }

    /// The folder a recorded removal happened in: the one at `stored`'s folder if it is still the
    /// same folder (`dir_id`); otherwise (moved after a crash) the folder in this destination that
    /// holds exactly the private name `private`, searched without following links and within a
    /// bound. `None` if neither is found.
    fn locate(
        &self,
        stored: &str,
        dir_id: FileId,
        private: &str,
    ) -> io::Result<Option<(Dir, String)>> {
        let folder = stored.rsplit_once('/').map_or("", |(f, _)| f).to_string();
        if let Some((dir, _)) = self.open_stored_folder(stored)?
            && identity(&dir.try_clone()?.into_std_file()).is_ok_and(|(id, _)| id == dir_id)
        {
            return Ok(Some((dir, folder)));
        }
        let mut queue = std::collections::VecDeque::from([(self.root.try_clone()?, String::new())]);
        let mut seen = 0;
        while let Some((dir, path)) = queue.pop_front() {
            seen += 1;
            if seen > MOST_FOLDERS_SEARCHED {
                break;
            }
            if rustix::fs::statat(&dir, private, AtFlags::SYMLINK_NOFOLLOW).is_ok() {
                return Ok(Some((dir, path)));
            }
            let Ok(entries) = dir.entries() else {
                continue;
            };
            for entry in entries.flatten() {
                // Only real folders: a link is never followed.
                if !entry
                    .file_type()
                    .is_ok_and(|k| k.is_dir() && !k.is_symlink())
                {
                    continue;
                }
                let Some(n) = entry.file_name().to_str().map(str::to_string) else {
                    continue;
                };
                if let Ok(sub) = dir.open_dir(&n) {
                    queue.push_back((sub, crate::stored_path(&path, &n)));
                }
            }
        }
        Ok(None)
    }
}

/// Puts whatever is under `private` back, and says how that ended: `home` under its own name, or
/// kept under a visible name beside it.
fn put_back_as(
    dir: &Dir,
    folder: &str,
    private: &str,
    name: &str,
    home: Resolution,
    acts: &Acts<'_>,
    cx: &mut Context<'_>,
) -> Result<Resolution, GateError> {
    Ok(match put_back(dir, private, name, acts, cx)? {
        Back::Home => home,
        Back::Beside(n) => Resolution::KeptAt {
            at: crate::stored_path(folder, &n),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn always() -> bool {
        true
    }

    fn names_in(dir: &std::path::Path) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        v.sort();
        v
    }

    /// Saved bytes keep the file's permissions, modified time and extended attributes, go under
    /// a visible name beside it (never over the person's file), and leave nothing hidden. The
    /// salvage is recorded as not complete before the copy and complete only after it.
    #[test]
    fn a_salvage_keeps_bytes_permissions_time_and_attributes_beside_the_name() {
        let root = tempfile::tempdir().unwrap();
        let p = root.path().join("a.txt");
        std::fs::write(&p, b"late bytes").unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o640)).unwrap();
        let old = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_600_000_000);
        std::fs::File::options()
            .write(true)
            .open(&p)
            .unwrap()
            .set_modified(old)
            .unwrap();
        let held = std::fs::File::open(&p).unwrap();
        let attr = rustix::fs::fsetxattr(
            &held,
            "user.pctwin",
            b"kept",
            rustix::fs::XattrFlags::empty(),
        )
        .is_ok();
        // The person's file is still under the name: the bytes go beside it.
        let dir = Dir::open_ambient_dir(root.path(), cap_std::ambient_authority()).unwrap();
        let mut steps = Vec::new();
        let mut journal = |s: Step<'_>| {
            steps.push(s.clone().into_owned());
            Ok(())
        };
        let mut cx = Context {
            kept_words: " (kept by PCTwin undo)",
            room_for_words: 64,
            others_closed: false,
            journal: &mut journal,
        };
        let acts = Acts { in_force: &always };
        let to = salvage(&dir, &held, "a.txt", Some("a.txt"), &acts, &mut cx).unwrap();
        assert_eq!(to, "a (kept by PCTwin undo).txt");
        let saved = root.path().join(&to);
        assert_eq!(std::fs::read(&saved).unwrap(), b"late bytes");
        let meta = std::fs::metadata(&saved).unwrap();
        assert_eq!(meta.permissions().mode() & 0o7777, 0o640);
        assert_eq!(meta.modified().unwrap(), old);
        if attr {
            let mut v = [0u8; 16];
            let f = std::fs::File::open(&saved).unwrap();
            let n = rustix::fs::fgetxattr(&f, "user.pctwin", &mut v[..]).unwrap();
            assert_eq!(&v[..n], b"kept");
        }
        assert_eq!(std::fs::read(&p).unwrap(), b"late bytes");
        assert!(
            names_in(root.path())
                .iter()
                .all(|n| !n.starts_with(".pctwin-")),
            "{:?}",
            names_in(root.path())
        );
        // Not complete before the copy; complete (for each name tried) only after it.
        let complete: Vec<(String, bool)> = steps
            .iter()
            .map(|s| match s {
                OwnedStep::Salvaging { to, complete, .. } => (to.clone(), *complete),
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(
            complete,
            [
                ("a.txt".to_string(), false),
                ("a.txt".to_string(), true),
                (to.clone(), true)
            ]
        );
    }

    /// A salvage stopped while copying (its undo right ended) leaves its temporary file with no
    /// permissions at all and is never recorded complete or named (finding 1).
    #[test]
    fn a_salvage_cut_short_has_no_permissions_and_is_never_recorded_complete() {
        let root = tempfile::tempdir().unwrap();
        let p = root.path().join("a.txt");
        std::fs::write(&p, vec![7u8; 300_000]).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o644)).unwrap();
        let held = std::fs::File::open(&p).unwrap();
        let dir = Dir::open_ambient_dir(root.path(), cap_std::ambient_authority()).unwrap();
        let mut steps = Vec::new();
        let mut journal = |s: Step<'_>| {
            steps.push(s.clone().into_owned());
            Ok(())
        };
        let mut cx = Context {
            kept_words: " (kept by PCTwin undo)",
            room_for_words: 64,
            others_closed: false,
            journal: &mut journal,
        };
        // The right ends once the temporary file exists and the first part is copied.
        let looked = std::cell::Cell::new(0);
        let path = root.path().to_path_buf();
        let in_force = || {
            let temp_made = names_in(&path)
                .iter()
                .any(|n| n.starts_with(".pctwin-salvage-"));
            if temp_made {
                looked.set(looked.get() + 1);
            }
            looked.get() < 2
        };
        let acts = Acts {
            in_force: &in_force,
        };
        assert!(salvage(&dir, &held, "a.txt", None, &acts, &mut cx).is_err());
        let temps: Vec<String> = names_in(root.path())
            .into_iter()
            .filter(|n| n.starts_with(".pctwin-salvage-"))
            .collect();
        assert_eq!(temps.len(), 1, "{:?}", names_in(root.path()));
        let mode = std::fs::symlink_metadata(root.path().join(&temps[0]))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(
            mode & 0o7777,
            0,
            "a temporary file cut short has no permissions"
        );
        assert!(
            steps.iter().all(|s| matches!(
                s,
                OwnedStep::Salvaging {
                    complete: false,
                    ..
                }
            )),
            "{steps:?}"
        );
        assert_eq!(names_in(root.path()).len(), 2);
    }

    /// The one way a private name is removed proves everything again right before it: a change
    /// of any kind since the last look (its change time included), or something else under the
    /// private name, removes nothing (finding 4).
    #[test]
    fn a_private_name_is_removed_only_after_proving_it_again_right_before() {
        let root = tempfile::tempdir().unwrap();
        let dir = Dir::open_ambient_dir(root.path(), cap_std::ambient_authority()).unwrap();
        let private = private_name().unwrap();
        let p = root.path().join(&private);
        let mut journal = |_: Step<'_>| Ok(());
        let mut cx = Context {
            kept_words: " (kept by PCTwin undo)",
            room_for_words: 64,
            others_closed: false,
            journal: &mut journal,
        };
        let acts = Acts { in_force: &always };
        // Permissions changed after the last look: only the change time tells.
        std::fs::write(&p, b"copy").unwrap();
        let held = std::fs::File::open(&p).unwrap();
        let expect = Look::of(&held).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600)).unwrap();
        let r = finish(
            &dir, &held, "a.txt", &private, expect, false, &acts, &mut cx,
        )
        .unwrap();
        assert_eq!(r, Finish::NotProven(Unproven::Changed));
        assert!(p.exists());
        // Another file put under the private name, the held one looking just as last looked at:
        // only the proof of what is under the name tells, and nothing is removed.
        std::fs::write(root.path().join("theirs"), b"theirs").unwrap();
        std::fs::rename(root.path().join("theirs"), &p).unwrap();
        let expect = Look::of(&held).unwrap();
        let r = finish(
            &dir, &held, "a.txt", &private, expect, false, &acts, &mut cx,
        )
        .unwrap();
        assert_eq!(r, Finish::NotProven(Unproven::NotThatFile));
        assert_eq!(std::fs::read(&p).unwrap(), b"theirs");
        // Nothing there any more.
        std::fs::remove_file(&p).unwrap();
        let r = finish(
            &dir, &held, "a.txt", &private, expect, false, &acts, &mut cx,
        )
        .unwrap();
        assert_eq!(r, Finish::NotProven(Unproven::Gone));
        // The very file, exactly as last looked at: removed.
        std::fs::write(&p, b"copy").unwrap();
        let held = std::fs::File::open(&p).unwrap();
        let expect = Look::of(&held).unwrap();
        let r = finish(
            &dir, &held, "a.txt", &private, expect, false, &acts, &mut cx,
        )
        .unwrap();
        assert_eq!(r, Finish::Removed);
        assert!(names_in(root.path()).is_empty());
        // A right that has ended removes nothing.
        std::fs::write(&p, b"copy").unwrap();
        let held = std::fs::File::open(&p).unwrap();
        let expect = Look::of(&held).unwrap();
        let ended = || false;
        let acts = Acts { in_force: &ended };
        assert!(
            finish(
                &dir, &held, "a.txt", &private, expect, false, &acts, &mut cx
            )
            .is_err()
        );
        assert!(p.exists());
    }

    #[test]
    fn only_exact_private_names_are_recognised() {
        assert!(is_private_name(&private_name().unwrap()));
        assert!(is_private_name(&format!(
            ".pctwin-salvage-{}",
            "a".repeat(32)
        )));
        assert!(!is_private_name(".pctwin-undo-"));
        assert!(!is_private_name(&format!(
            ".pctwin-undo-{}",
            "A".repeat(32)
        )));
        assert!(!is_private_name(&format!(
            ".pctwin-undo-{}x",
            "a".repeat(32)
        )));
        assert!(!is_private_name("a.txt"));
    }

    /// A step as kept by a test journal.
    #[derive(Debug)]
    enum OwnedStep {
        Salvaging { to: String, complete: bool },
        Other,
    }

    impl Step<'_> {
        fn into_owned(self) -> OwnedStep {
            match self {
                Step::Salvaging { to, complete, .. } => OwnedStep::Salvaging {
                    to: to.to_string(),
                    complete,
                },
                _ => OwnedStep::Other,
            }
        }
    }
}
