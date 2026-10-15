//! How a drive gives a finished file its real name without ever replacing anything, found once
//! per drive by trying it on PCTwin's own probe files (security design notes, "Naming a finished
//! file never replaces anything" and "Drives with no safe rename", decided 15 October 2026).
//!
//! The test, in a folder of the drive:
//! 1. Probe files a crash left behind (`.pctwin-probe-<random>-a`, `-b`, `-c`) are removed, but
//!    only those holding the probe's own bytes; anything else with such a name stays.
//! 2. Two probe files `a` and `b` are made, each only if nothing has its name; making `a` again
//!    must then be refused (exclusive create works).
//! 3. A hard link from `a` to a new name `c`, and from `a` onto `b`, which must be refused.
//! 4. A no-replace move of `a` onto `b`: refused because `b` exists ("exists") means the drive
//!    honours it; refused as unsupported ("invalid", "not implemented", "not supported") means
//!    it has no such move; done, with `b` silently replaced, means it ignores the request, which
//!    is recorded ([`DriveCheck::replaced_silently`]) for the report.
//! 5. All probe files are removed.
//!
//! A hard link or an honoured no-replace move makes the whole file appear at once
//! ([`Naming::Atomic`]). Without either, the name is created only if free and then filled
//! ([`Naming::FillIn`]); a drive where even an exclusive create is not honoured has no safe way
//! at all ([`Naming::NoSafeMethod`]) and is refused when it is chosen. Each error number is read
//! only against this result: an "invalid" from a drive whose test passed is the name itself.

use std::hash::{BuildHasher, RandomState};
use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};

use cap_std::fs::{Dir, OpenOptions};

use crate::FolderPath;

/// How a drive gives a finished file its real name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Naming {
    /// A hard link, or a move the drive refuses when the name exists: the whole file appears at
    /// once, or not at all.
    Atomic,
    /// Neither: the real name is created only if nothing has it, then filled from the checked
    /// temporary file (exFAT on a Mac). Slower, and it needs room for the file twice.
    FillIn,
    /// Not even a name made only if free can be trusted here: the drive is refused.
    NoSafeMethod,
}

/// What testing a drive found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DriveCheck {
    pub naming: Naming,
    /// A hard link to a new name works, and one onto an existing name is refused.
    pub hard_links: bool,
    /// A move onto an existing name is refused when asked not to replace.
    pub no_replace_move: bool,
    /// Asked to move without replacing, the drive replaced the file anyway: it ignores the
    /// request. Never used for naming; the report can say so.
    pub replaced_silently: bool,
    /// Making a file only if nothing has its name is honoured.
    pub exclusive_create: bool,
}

impl DriveCheck {
    /// What a forced way of naming (for tests, through a drive profile) stands for.
    pub(crate) fn forced(naming: Naming, hard_links: bool) -> Self {
        let atomic = naming == Naming::Atomic;
        Self {
            naming,
            hard_links: atomic && hard_links,
            no_replace_move: atomic && !hard_links,
            replaced_silently: false,
            exclusive_create: naming != Naming::NoSafeMethod,
        }
    }
}

/// How a no-replace move ended, as the drive said it.
#[derive(Debug)]
pub(crate) enum Moved {
    /// Moved: nothing had the name (or the drive replaced it, ignoring the request).
    Done,
    /// Refused because something has the name.
    Taken,
    /// Refused as invalid: on a drive whose test passed, the name; in the test, no such move.
    /// (Linux and macOS; Windows says which it is.)
    #[cfg_attr(
        not(unix),
        allow(dead_code, reason = "only Linux and macOS answer this way")
    )]
    Invalid,
    /// Refused as not implemented or not supported: the drive has no such move.
    Unsupported,
    /// Refused because of the name itself (too long, characters the drive cannot hold).
    Name,
    /// Anything else.
    Failed(io::Error),
}

/// What a drive's test found, from what each step said. Only the probe's own files are touched.
pub(crate) fn classify(
    link_new: bool,
    link_onto_existing_refused: bool,
    moved: &Moved,
    b_intact: bool,
    exclusive_create: bool,
) -> DriveCheck {
    let hard_links = link_new && link_onto_existing_refused;
    let no_replace_move = matches!(moved, Moved::Taken) && b_intact;
    let replaced_silently =
        matches!(moved, Moved::Done) || !b_intact || (link_new && !link_onto_existing_refused);
    let naming = if !exclusive_create {
        Naming::NoSafeMethod
    } else if hard_links || no_replace_move {
        Naming::Atomic
    } else {
        Naming::FillIn
    };
    DriveCheck {
        naming,
        hard_links,
        no_replace_move,
        replaced_silently,
        exclusive_create,
    }
}

/// What the first and second probe files hold.
pub(crate) const PROBE_A: &[u8] = b"PCTwin drive test: file a. Safe to delete.\n";
pub(crate) const PROBE_B: &[u8] = b"PCTwin drive test: file b. Safe to delete.\n";
const PROBE_PREFIX: &str = ".pctwin-probe-";
/// Longest random part of a probe name accepted when clearing leftovers.
const MAX_PROBE_TAG: usize = 32;

static PROBES: AtomicU64 = AtomicU64::new(0);

/// Whether `name` is one of the probe's names (`.pctwin-probe-<tag>-a`, `-b` or `-c`).
fn is_probe_name(name: &str) -> bool {
    name.strip_prefix(PROBE_PREFIX)
        .and_then(|rest| {
            rest.strip_suffix("-a")
                .or_else(|| rest.strip_suffix("-b"))
                .or_else(|| rest.strip_suffix("-c"))
        })
        .is_some_and(|tag| {
            !tag.is_empty()
                && tag.len() <= MAX_PROBE_TAG
                && tag.chars().all(|c| c.is_ascii_alphanumeric())
        })
}

/// Removes the probe file `name` in `dir` if it is a regular file holding exactly the probe's
/// own bytes; anything else is left. Never follows a link.
fn remove_if_probe(dir: &Dir, name: &str) -> io::Result<()> {
    let meta = match dir.symlink_metadata(name) {
        Ok(meta) => meta,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    let longest = PROBE_A.len().max(PROBE_B.len()) as u64;
    if !meta.is_file() || meta.len() > longest {
        return Ok(());
    }
    let mut bytes = Vec::new();
    dir.open(name)?.take(longest + 1).read_to_end(&mut bytes)?;
    if bytes != PROBE_A && bytes != PROBE_B {
        return Ok(());
    }
    match dir.remove_file(name) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

/// Makes `name` in `dir` holding `bytes`, only if nothing has the name.
fn make(dir: &Dir, name: &str, bytes: &[u8]) -> io::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    let mut file = dir.open_with(name, &options)?;
    file.write_all(bytes)?;
    file.sync_all()
}

/// Tests the drive `dir` is on (see the module notes). `move_no_replace` is the system's
/// no-replace move; `folder_path` gives the folder's full path where that move needs one.
pub(crate) fn probe(
    dir: &Dir,
    folder_path: FolderPath<'_>,
    move_no_replace: fn(&Dir, &str, &str, FolderPath<'_>) -> Moved,
) -> io::Result<DriveCheck> {
    for entry in dir.entries()? {
        let name = entry?.file_name();
        if let Some(name) = name.to_str().filter(|n| is_probe_name(n)) {
            remove_if_probe(dir, name)?;
        }
    }
    let n = PROBES.fetch_add(1, Ordering::Relaxed);
    let tag = RandomState::new().hash_one((std::process::id(), n));
    let [a, b, c] = ["a", "b", "c"].map(|s| format!("{PROBE_PREFIX}{tag:016x}-{s}"));
    let result = run(dir, [&a, &b, &c], folder_path, move_no_replace);
    // Every probe file goes, whatever the test found.
    let mut cleared = Ok(());
    for name in [&a, &b, &c] {
        if let Err(e) = remove_if_probe(dir, name) {
            cleared = Err(e);
        }
    }
    let check = result?;
    cleared?;
    Ok(check)
}

fn run(
    dir: &Dir,
    [a, b, c]: [&str; 3],
    folder_path: FolderPath<'_>,
    move_no_replace: fn(&Dir, &str, &str, FolderPath<'_>) -> Moved,
) -> io::Result<DriveCheck> {
    make(dir, a, PROBE_A)?;
    make(dir, b, PROBE_B)?;
    let exclusive_create = match make(dir, a, PROBE_B) {
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => true,
        Err(e) => return Err(e),
        // Made over an existing file: exclusive create is not honoured. Put `a` back as it was.
        Ok(()) => {
            dir.remove_file(a)?;
            make(dir, a, PROBE_A)?;
            false
        }
    };
    let link_new = dir.hard_link(a, dir, c).is_ok();
    let link_onto_existing_refused = link_new
        && matches!(dir.hard_link(a, dir, b), Err(e) if e.kind() == io::ErrorKind::AlreadyExists);
    if link_new {
        // The link is another name for `a`'s bytes: removed with the rest.
        dir.remove_file(c)?;
    }
    let moved = move_no_replace(dir, a, b, folder_path);
    if let Moved::Failed(e) = moved {
        return Err(e);
    }
    let mut now = Vec::new();
    dir.open(b)?.read_to_end(&mut now)?;
    Ok(classify(
        link_new,
        link_onto_existing_refused,
        &moved,
        now == PROBE_B,
        exclusive_create,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_each_answer_means() {
        use Naming::*;
        let err = || Moved::Failed(io::Error::other("x"));
        // (link to a new name, link onto b refused, the move, b intact, exclusive create)
        for (link_new, refused, moved, intact, excl, naming, hard, mv, silent) in [
            // NTFS, ext4, APFS: hard links.
            (
                true,
                true,
                Moved::Taken,
                true,
                true,
                Atomic,
                true,
                true,
                false,
            ),
            // FAT, exFAT on Linux and Windows: only the no-replace move.
            (
                false,
                false,
                Moved::Taken,
                true,
                true,
                Atomic,
                false,
                true,
                false,
            ),
            // exFAT on a Mac: neither.
            (
                false,
                false,
                Moved::Invalid,
                true,
                true,
                FillIn,
                false,
                false,
                false,
            ),
            (
                false,
                false,
                Moved::Unsupported,
                true,
                true,
                FillIn,
                false,
                false,
                false,
            ),
            // A drive that replaces anyway: never trusted to move.
            (
                false,
                false,
                Moved::Done,
                false,
                true,
                FillIn,
                false,
                false,
                true,
            ),
            // "Exists", yet b changed: not trusted either.
            (
                false,
                false,
                Moved::Taken,
                false,
                true,
                FillIn,
                false,
                false,
                true,
            ),
            // A hard link that went onto an existing name.
            (
                true,
                false,
                Moved::Invalid,
                true,
                true,
                FillIn,
                false,
                false,
                true,
            ),
            // Hard links work though the move is unsupported.
            (
                true,
                true,
                Moved::Unsupported,
                true,
                true,
                Atomic,
                true,
                false,
                false,
            ),
            // Exclusive create not honoured: nothing is safe.
            (
                false,
                false,
                Moved::Invalid,
                true,
                false,
                NoSafeMethod,
                false,
                false,
                false,
            ),
            (
                true,
                true,
                Moved::Taken,
                true,
                false,
                NoSafeMethod,
                true,
                true,
                false,
            ),
            // A name problem in the test means no usable move.
            (
                false,
                false,
                Moved::Name,
                true,
                true,
                FillIn,
                false,
                false,
                false,
            ),
        ] {
            let c = classify(link_new, refused, &moved, intact, excl);
            assert_eq!(
                (
                    c.naming,
                    c.hard_links,
                    c.no_replace_move,
                    c.replaced_silently
                ),
                (naming, hard, mv, silent),
                "{link_new} {refused} {moved:?} {intact} {excl}"
            );
            assert_eq!(c.exclusive_create, excl);
        }
        // A step that failed for another reason is no answer at all (the test is run again).
        assert!(!classify(false, false, &err(), true, true).no_replace_move);
    }

    #[test]
    fn probe_names_are_recognised_and_nothing_else() {
        for yes in [
            ".pctwin-probe-0a-a",
            ".pctwin-probe-ff00ff-b",
            ".pctwin-probe-1-c",
        ] {
            assert!(is_probe_name(yes), "{yes}");
        }
        for no in [
            ".pctwin-probe--a",
            ".pctwin-probe-0a-d",
            ".pctwin-probe-0a",
            ".pctwin-probe-0/a-a",
            ".pctwin-probe-0.-a",
            "pctwin-probe-0-a",
            ".pctwin-0-a.part",
            &format!(".pctwin-probe-{}-a", "0".repeat(33)),
        ] {
            assert!(!is_probe_name(no), "{no}");
        }
    }
}
