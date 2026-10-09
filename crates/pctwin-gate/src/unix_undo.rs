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
//!    showing the file save and let go; steps 6 to 9 run while they wait), then the system's
//!    list of programs with the file open; if either cannot be asked, nothing is removed.
//! 6. The name moves to the private name, never replacing.
//! 7. What is under the private name is proven to be the held file, unchanged; else it goes back.
//! 8. The private name is removed (Linux: only while the lease shows nobody waiting).
//! 9. The held file is checked once more: no names left and unchanged is removed; anything
//!    written meanwhile is saved beside it under a visible name.

use std::io;

use rustix::fs::{AtFlags, Mode, OFlags, RenameFlags};
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
    /// About to save bytes into the new private file `temp`, to be named `to`.
    Salvaging { temp: &'a str, to: &'a str },
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
    /// with PCTwin's own rename and removal, so it is not compared after those).
    fn same_contents(&self, other: &Look) -> bool {
        self.len == other.len && self.modified == other.modified
    }
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
        if rustix::fs::FileType::from_raw_mode(st.st_mode) != rustix::fs::FileType::RegularFile {
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
    /// an open undo permit, so nothing can be removed once undo is closed for good. The folder is
    /// not flushed here: the caller flushes each folder touched ([`flush_folder`](Self::flush_folder))
    /// before recording the batch as done.
    pub fn remove_checked(
        &self,
        _right: &impl pctwin_journal::UndoRight,
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
        // 5. Nobody else has it open.
        #[cfg(target_os = "macos")]
        {
            let Some(path) = self.root_path.as_ref().map(|r| r.join(&folder).join(&name)) else {
                return Ok(Removed::CannotCheck);
            };
            // Apps presenting the file are told and let go; the rest happens while they wait.
            let done = macos_check::coordinated_for_deleting(&path, || {
                match macos_check::others_have_it_open(&path) {
                    Ok(true) => Ok(Removed::InUse),
                    Err(_) => Ok(Removed::CannotCheck),
                    Ok(false) => {
                        move_prove_remove(&dir, &folder, &name, &handle, look0, private, false, cx)
                    }
                }
            });
            done.unwrap_or(Ok(Removed::CannotCheck))
        }
        #[cfg(not(target_os = "macos"))]
        {
            let leased = match others_have_it_open(&handle, cx.others_closed) {
                Openers::None { leased } => leased,
                Openers::Some => return Ok(Removed::InUse),
                Openers::CannotCheck => return Ok(Removed::CannotCheck),
            };
            let r = move_prove_remove(&dir, &folder, &name, &handle, look0, private, leased, cx);
            #[cfg(target_os = "linux")]
            if leased {
                let _ = pctwin_lease::release(&handle);
            }
            r
        }
    }

    /// Flushes the folder at the stored path `folder` ("" for the approved folder itself), so
    /// every removal in it is on the disk before the journal says it is done.
    pub fn flush_folder(&self, folder: &str) -> io::Result<()> {
        let dir = if folder.is_empty() {
            self.root.try_clone()?
        } else {
            let probe = format!("{folder}/x");
            match self.open_stored_folder(&probe)? {
                Some((dir, _)) => dir,
                None => return Err(io::Error::from(io::ErrorKind::NotFound)),
            }
        };
        sync_folder(&dir)
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
        match rustix::fs::renameat_with(dir, name, dir, private.as_str(), RenameFlags::NOREPLACE) {
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
    let moved = rustix::fs::statat(dir, private.as_str(), AtFlags::SYMLINK_NOFOLLOW);
    let held = rustix::fs::fstat(handle)?;
    let ours = moved
        .as_ref()
        .is_ok_and(|m| m.st_dev == held.st_dev && m.st_ino == held.st_ino);
    let now = Look::from(&held);
    if !ours || now.names != 1 || !now.same_contents(&look0) {
        return Ok(match put_back(dir, &private, name, cx)? {
            Back::Home if !ours => Removed::NotThatFile,
            Back::Home => Removed::Changed,
            Back::Beside(n) => Removed::KeptBeside { at: at(&n) },
        });
    }
    // 8. Removed, only while nobody has started to open it.
    if leased && !lease_still_held(handle) {
        return Ok(match put_back(dir, &private, name, cx)? {
            Back::Home => Removed::InUse,
            Back::Beside(n) => Removed::KeptBeside { at: at(&n) },
        });
    }
    match rustix::fs::unlinkat(dir, private.as_str(), AtFlags::empty()) {
        Ok(()) | Err(Errno::NOENT) => {}
        Err(e) => return Err(e.into()),
    }
    // 9. No names left and unchanged. Bytes written meanwhile are saved beside it; a program
    // that started to open it at the last moment expects it under its own name, so a copy goes
    // back there if that is free.
    let after = Look::of(handle)?;
    let opened_meanwhile = leased && !lease_still_held(handle);
    if after.names == 0 && after.same_contents(&look0) && !opened_meanwhile {
        return Ok(Removed::Removed);
    }
    let first = opened_meanwhile.then_some(name);
    let to = salvage(dir, handle, name, first, cx)?;
    Ok(Removed::Salvaged { at: at(&to) })
}

/// Removes the proven copy under `private` (held as `held`, looked at as `before`), then makes sure
/// nothing written to it meanwhile is lost: `None` if removed cleanly, or the visible name the late
/// bytes were saved under.
fn finish_unlink(
    dir: &Dir,
    held: &std::fs::File,
    name: &str,
    private: &str,
    before: Look,
    leased: bool,
    cx: &mut Context<'_>,
) -> Result<Option<String>, GateError> {
    match rustix::fs::unlinkat(dir, private, AtFlags::empty()) {
        Ok(()) | Err(Errno::NOENT) => {}
        Err(e) => return Err(e.into()),
    }
    let after = Look::of(held)?;
    let opened_meanwhile = leased && !lease_still_held(held);
    if after.names == 0 && after.same_contents(&before) && !opened_meanwhile {
        return Ok(None);
    }
    let first = opened_meanwhile.then_some(name);
    Ok(Some(salvage(dir, held, name, first, cx)?))
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
fn put_back(dir: &Dir, private: &str, name: &str, cx: &mut Context<'_>) -> Result<Back, GateError> {
    (cx.journal)(Step::Putting { private, to: name })?;
    match rustix::fs::renameat_with(dir, private, dir, name, RenameFlags::NOREPLACE) {
        Ok(()) => return Ok(Back::Home),
        Err(Errno::EXIST) => {}
        Err(e) => return Err(GateError::Io(e.into())),
    }
    for n in 1..=MOST_TRIES {
        let to = sibling_name(name, cx.kept_words, n, cx.room_for_words);
        (cx.journal)(Step::Putting { private, to: &to })?;
        match rustix::fs::renameat_with(dir, private, dir, to.as_str(), RenameFlags::NOREPLACE) {
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
/// file, then that is named without replacing anything.
fn salvage(
    dir: &Dir,
    held: &std::fs::File,
    name: &str,
    first: Option<&str>,
    cx: &mut Context<'_>,
) -> Result<String, GateError> {
    let temp = salvage_name()?;
    let mut targets: Vec<String> = first.map(str::to_string).into_iter().collect();
    targets
        .extend((1..=MOST_TRIES).map(|n| sibling_name(name, cx.kept_words, n, cx.room_for_words)));
    (cx.journal)(Step::Salvaging {
        temp: &temp,
        to: &targets[0],
    })?;
    let fd = rustix::fs::openat(
        dir,
        temp.as_str(),
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::RUSR | Mode::WUSR,
    )?;
    let mut out = std::fs::File::from(fd);
    copy_from(held, &mut out)?;
    let st = rustix::fs::fstat(held)?;
    rustix::fs::fchmod(
        &out,
        Mode::from_raw_mode(st.st_mode) & Mode::from_bits_truncate(0o7777),
    )?;
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
    out.sync_all()?;
    for (i, to) in targets.iter().enumerate() {
        if i > 0 {
            (cx.journal)(Step::Salvaging { temp: &temp, to })?;
        }
        match rustix::fs::renameat_with(
            dir,
            temp.as_str(),
            dir,
            to.as_str(),
            RenameFlags::NOREPLACE,
        ) {
            Ok(()) => return Ok(to.clone()),
            Err(Errno::EXIST) => continue,
            Err(e) => return Err(GateError::Io(e.into())),
        }
    }
    Err(GateError::TooManyClashes)
}

/// Copies every byte of `from` (read from its start, by position) into `to`.
fn copy_from(from: &std::fs::File, to: &mut std::fs::File) -> io::Result<()> {
    use std::io::Write;
    let mut buf = vec![0u8; 1 << 16];
    let mut at = 0u64;
    loop {
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
/// at the next start and again when undo closes). Every answer is decided from the disk and is
/// safe to reach again: running a resolution twice ends the same way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    /// The very file, unchanged, was under its private name and is now removed.
    Removed,
    /// Nothing of it is left under its name or its private name: it was already removed (or
    /// moved away by the person).
    AlreadyGone,
    /// Nothing was moved: the copy is still under its own name (undo stopped before moving it).
    StillThere,
    /// Put back under its own name, being changed or not PCTwin's copy.
    Home,
    /// Put back under its own name: another program has it open (try again later).
    InUse,
    /// Put back under its own name: the system cannot say whether another program has it open,
    /// and the person has not confirmed their other programs are closed.
    CannotCheck,
    /// Kept under the visible name `at` beside its own (the stored path).
    KeptAt { at: String },
    /// Its folder cannot be found on this drive (unplugged, or moved somewhere undo cannot find
    /// it): not resolved, nothing changed.
    FolderMissing,
}

/// How many folders the search for a folder moved after a crash looks in, at most.
const MOST_FOLDERS_SEARCHED: usize = 20_000;

impl Destination {
    /// Finishes a removal recorded as `Removing { file, dir_id, private }` for the copy at the
    /// stored path `stored`, with the caller's check `verify` for "the very file, unchanged".
    #[expect(
        clippy::too_many_arguments,
        reason = "exactly the journal's record and the check"
    )]
    pub fn resolve_removing(
        &self,
        _right: &impl pctwin_journal::UndoRight,
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
        let name = stored.rsplit('/').next().unwrap_or(stored);
        let Some((dir, folder)) = self.locate(stored, dir_id, private)? else {
            return Ok(Resolution::FolderMissing);
        };
        let at = |n: &str| crate::stored_path(&folder, n);
        let Some(mut held) = open_no_follow(&dir, private)? else {
            // Nothing under the private name: either it was never moved, or already removed.
            return Ok(match open_no_follow(&dir, name)? {
                Some(f) if identity(&f).is_ok_and(|(id, _)| id == file) => Resolution::StillThere,
                _ => Resolution::AlreadyGone,
            });
        };
        let before = Look::of(&held)?;
        let ours = identity(&held).is_ok_and(|(id, names)| id == file && names == 1);
        let mut kept = Resolution::Home;
        if ours && verify(&mut held)? && Look::of(&held)? == before {
            // A program that had it open before PCTwin stopped may still be writing to it: such
            // a file goes back under its name for another try, as at any other time.
            match self.nobody_else_has(&held, &folder, private, cx.others_closed) {
                Openers::None { leased } => {
                    let r = finish_unlink(&dir, &held, name, private, before, leased, cx);
                    #[cfg(target_os = "linux")]
                    if leased {
                        let _ = pctwin_lease::release(&held);
                    }
                    let r = r?;
                    sync_folder(&dir)?;
                    return Ok(match r {
                        None => Resolution::Removed,
                        Some(to) => Resolution::KeptAt { at: at(&to) },
                    });
                }
                Openers::Some => kept = Resolution::InUse,
                Openers::CannotCheck => kept = Resolution::CannotCheck,
            }
        }
        drop(held);
        let back = put_back(&dir, private, name, cx)?;
        sync_folder(&dir)?;
        Ok(match back {
            Back::Home => kept,
            Back::Beside(n) => Resolution::KeptAt { at: at(&n) },
        })
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

    /// Finishes a recorded `Putting { private, to }`: whatever is under `private` goes to `to`
    /// (or the next free visible name), or is confirmed already there.
    pub fn resolve_putting(
        &self,
        _right: &impl pctwin_journal::UndoRight,
        stored: &str,
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
        let name = stored.rsplit('/').next().unwrap_or(stored);
        let Some((dir, folder)) = self.locate(stored, dir_id, private)? else {
            return Ok(Resolution::FolderMissing);
        };
        let at = |n: &str| crate::stored_path(&folder, n);
        let resolved = |to: &str| {
            if to == name {
                Resolution::Home
            } else {
                Resolution::KeptAt { at: at(to) }
            }
        };
        if open_no_follow(&dir, private)?.is_none() {
            // Already moved (the rename is all or nothing): it is under `to`, or moved on since.
            return Ok(
                match rustix::fs::statat(&dir, to, AtFlags::SYMLINK_NOFOLLOW) {
                    Ok(_) => resolved(to),
                    Err(_) => Resolution::AlreadyGone,
                },
            );
        }
        match rustix::fs::renameat_with(&dir, private, &dir, to, RenameFlags::NOREPLACE) {
            Ok(()) => {
                sync_folder(&dir)?;
                return Ok(resolved(to));
            }
            Err(Errno::EXIST) => {}
            Err(e) => return Err(e.into()),
        }
        let back = put_back(&dir, private, name, cx)?;
        sync_folder(&dir)?;
        Ok(match back {
            Back::Home => Resolution::Home,
            Back::Beside(n) => Resolution::KeptAt { at: at(&n) },
        })
    }

    /// Finishes a recorded `Salvaging { temp, to }`: the saved bytes (perhaps not all of them,
    /// if PCTwin stopped while copying) are given a visible name, never replacing anything.
    pub fn resolve_salvaging(
        &self,
        _right: &impl pctwin_journal::UndoRight,
        stored: &str,
        dir_id: FileId,
        temp: &str,
        to: &str,
        cx: &mut Context<'_>,
    ) -> Result<Resolution, GateError> {
        if !is_private_name(temp) {
            return Err(GateError::Io(crate::invalid(
                "not one of undo's private names",
            )));
        }
        let name = stored.rsplit('/').next().unwrap_or(stored);
        let Some((dir, folder)) = self.locate(stored, dir_id, temp)? else {
            return Ok(Resolution::FolderMissing);
        };
        let at = |n: &str| crate::stored_path(&folder, n);
        if open_no_follow(&dir, temp)?.is_none() {
            return Ok(
                match rustix::fs::statat(&dir, to, AtFlags::SYMLINK_NOFOLLOW) {
                    Ok(_) => Resolution::KeptAt { at: at(to) },
                    Err(_) => Resolution::AlreadyGone,
                },
            );
        }
        let mut targets = vec![to.to_string()];
        for n in 1..=MOST_TRIES {
            let t = sibling_name(name, cx.kept_words, n, cx.room_for_words);
            if t != to {
                targets.push(t);
            }
        }
        for (i, target) in targets.iter().enumerate() {
            if i > 0 {
                (cx.journal)(Step::Salvaging { temp, to: target })?;
            }
            match rustix::fs::renameat_with(
                &dir,
                temp,
                &dir,
                target.as_str(),
                RenameFlags::NOREPLACE,
            ) {
                Ok(()) => {
                    sync_folder(&dir)?;
                    return Ok(Resolution::KeptAt { at: at(target) });
                }
                Err(Errno::EXIST) => continue,
                Err(e) => return Err(e.into()),
            }
        }
        Err(GateError::TooManyClashes)
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

/// Opens `name` in `dir` for reading without following a link or waiting on a pipe; `None` if
/// nothing (or not a regular file) is there.
fn open_no_follow(dir: &Dir, name: &str) -> Result<Option<std::fs::File>, GateError> {
    match rustix::fs::openat(
        dir,
        name,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::NOCTTY | OFlags::CLOEXEC,
        Mode::empty(),
    ) {
        Ok(fd) => {
            let f = std::fs::File::from(fd);
            let st = rustix::fs::fstat(&f)?;
            let regular = rustix::fs::FileType::from_raw_mode(st.st_mode)
                == rustix::fs::FileType::RegularFile;
            Ok(regular.then_some(f))
        }
        Err(Errno::NOENT | Errno::LOOP | Errno::NXIO | Errno::ISDIR) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Saved bytes keep the file's permissions, modified time and extended attributes, go under
    /// a visible name beside it (never over the person's file), and leave nothing hidden.
    #[test]
    fn a_salvage_keeps_bytes_permissions_time_and_attributes_beside_the_name() {
        use std::os::unix::fs::PermissionsExt;
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
            steps.push(format!("{s:?}"));
            Ok(())
        };
        let mut cx = Context {
            kept_words: " (kept by PCTwin undo)",
            room_for_words: 64,
            others_closed: false,
            journal: &mut journal,
        };
        let to = salvage(&dir, &held, "a.txt", Some("a.txt"), &mut cx).unwrap();
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
        let names: Vec<String> = std::fs::read_dir(root.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert!(
            names.iter().all(|n| !n.starts_with(".pctwin-")),
            "{names:?}"
        );
        assert_eq!(steps.len(), 2, "{steps:?}");
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
}
