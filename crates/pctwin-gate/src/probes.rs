//! Every probe file PCTwin makes on a drive to test it, in one list: the drive test's (`-a`,
//! `-b`, `-c`; the naming module), the time test's (`-t`, `-u`; the drive module) and the
//! read-back test's (`-r`). Each is named `.pctwin-probe-<random>-<kind>`, is made only if its
//! name is free, holds bytes of its own, and is removed only through [`remove_if_probe`]: a
//! regular file, never followed through a link, holding exactly its kind's own bytes; anything
//! else at the name stays. Every test first clears its folder of probe files a crash left behind
//! ([`sweep`]), only those older than [`LEFTOVER_AGE`], so a test running now from another handle
//! on the drive is never disturbed.

use std::hash::{BuildHasher, RandomState};
use std::io::{self, Read};
use std::sync::atomic::{AtomicU64, Ordering};

use cap_std::fs::Dir;

/// What the drive test's first and second probe files hold.
pub(crate) const DRIVE_A: &[u8] = b"PCTwin drive test: file a. Safe to delete.\n";
pub(crate) const DRIVE_B: &[u8] = b"PCTwin drive test: file b. Safe to delete.\n";
/// What the time test's probe files hold.
pub(crate) const TIME: &[u8] = b"PCTwin drive test: file t. Safe to delete.\n";
/// How long the read-back test's probe file is: two whole reads past the cache.
pub(crate) const READ_BACK_LEN: usize = 2 * crate::read_back::ALIGN;

/// What the read-back test's probe file holds: a fixed pattern no ordinary file has.
pub(crate) fn read_back_bytes() -> Vec<u8> {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "the remainder is below 251, so it fits a byte"
    )]
    (0..READ_BACK_LEN).map(|i| (i % 251) as u8 ^ 0x5A).collect()
}

const PREFIX: &str = ".pctwin-probe-";
/// Longest random part of a probe name accepted when clearing leftovers.
const MAX_TAG: usize = 32;

/// How old a probe file must be before it is taken as left by a crash: a test takes moments.
pub(crate) const LEFTOVER_AGE: std::time::Duration = std::time::Duration::from_secs(10 * 60);

/// Every kind of probe file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Probe {
    DriveA,
    DriveB,
    DriveC,
    TimeT,
    TimeU,
    ReadBack,
}

impl Probe {
    const ALL: [Self; 6] = [
        Self::DriveA,
        Self::DriveB,
        Self::DriveC,
        Self::TimeT,
        Self::TimeU,
        Self::ReadBack,
    ];

    fn suffix(self) -> &'static str {
        match self {
            Self::DriveA => "-a",
            Self::DriveB => "-b",
            Self::DriveC => "-c",
            Self::TimeT => "-t",
            Self::TimeU => "-u",
            Self::ReadBack => "-r",
        }
    }

    /// This probe's name for a test tagged `tag` (see [`new_tag`]).
    pub(crate) fn name(self, tag: u64) -> String {
        format!("{PREFIX}{tag:016x}{}", self.suffix())
    }

    /// Whether `bytes` are what a probe file of this kind holds. The drive test moves and links
    /// its files between its names, so each of its names may hold `a`'s bytes or `b`'s.
    fn holds(self, bytes: &[u8]) -> bool {
        match self {
            Self::DriveA | Self::DriveB | Self::DriveC => bytes == DRIVE_A || bytes == DRIVE_B,
            Self::TimeT | Self::TimeU => bytes == TIME,
            Self::ReadBack => bytes == read_back_bytes(),
        }
    }

    /// The longest a probe file of this kind is.
    fn longest(self) -> usize {
        match self {
            Self::DriveA | Self::DriveB | Self::DriveC => DRIVE_A.len().max(DRIVE_B.len()),
            Self::TimeT | Self::TimeU => TIME.len(),
            Self::ReadBack => READ_BACK_LEN,
        }
    }
}

/// A fresh random part for one test's probe names: different for every test in every program.
pub(crate) fn new_tag() -> u64 {
    static TESTS: AtomicU64 = AtomicU64::new(0);
    let n = TESTS.fetch_add(1, Ordering::Relaxed);
    RandomState::new().hash_one((std::process::id(), n))
}

/// The kind of probe `name` is named as, if it is one.
pub(crate) fn kind_of(name: &str) -> Option<Probe> {
    let rest = name.strip_prefix(PREFIX)?;
    Probe::ALL.into_iter().find(|kind| {
        rest.strip_suffix(kind.suffix()).is_some_and(|tag| {
            !tag.is_empty()
                && tag.len() <= MAX_TAG
                && tag.chars().all(|c| c.is_ascii_alphanumeric())
        })
    })
}

/// Removes the probe file `name` in `dir` if it is a regular file holding exactly its kind's own
/// bytes (and, for a leftover, older than [`LEFTOVER_AGE`]); anything else is left, and so is a
/// name that is not a probe's. Never follows a link.
pub(crate) fn remove_if_probe(dir: &Dir, name: &str, leftover: bool) -> io::Result<()> {
    let Some(kind) = kind_of(name) else {
        return Ok(());
    };
    let meta = match dir.symlink_metadata(name) {
        Ok(meta) => meta,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    if leftover {
        let modified = meta.modified()?.into_std();
        let old = std::time::SystemTime::now()
            .duration_since(modified)
            .is_ok_and(|age| age >= LEFTOVER_AGE);
        if !old {
            return Ok(());
        }
    }
    let longest = kind.longest() as u64;
    if !meta.is_file() || meta.len() > longest {
        return Ok(());
    }
    let mut bytes = Vec::new();
    dir.open(name)?.take(longest + 1).read_to_end(&mut bytes)?;
    if !kind.holds(&bytes) {
        return Ok(());
    }
    match dir.remove_file(name) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

/// Clears `dir` of probe files of every kind a crash left behind (see [`remove_if_probe`]).
pub(crate) fn sweep(dir: &Dir) -> io::Result<()> {
    for entry in dir.entries()? {
        let name = entry?.file_name();
        if let Some(name) = name.to_str().filter(|n| kind_of(n).is_some()) {
            remove_if_probe(dir, name, true)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_names_are_recognised_and_nothing_else() {
        for (yes, kind) in [
            (".pctwin-probe-0a-a", Probe::DriveA),
            (".pctwin-probe-ff00ff-b", Probe::DriveB),
            (".pctwin-probe-1-c", Probe::DriveC),
            (".pctwin-probe-1-t", Probe::TimeT),
            (".pctwin-probe-1-u", Probe::TimeU),
            (".pctwin-probe-1-r", Probe::ReadBack),
        ] {
            assert_eq!(kind_of(yes), Some(kind), "{yes}");
        }
        for no in [
            ".pctwin-probe--a",
            ".pctwin-probe-0a-d",
            ".pctwin-probe-0a",
            ".pctwin-probe-0/a-a",
            ".pctwin-probe-0.-a",
            "pctwin-probe-0-a",
            ".pctwin-0-a.part",
            ".pctwin-probe-1-2.part",
            &format!(".pctwin-probe-{}-a", "0".repeat(33)),
        ] {
            assert_eq!(kind_of(no), None, "{no}");
        }
    }

    /// Every probe maker names its files from this one list, so the sweep knows them all.
    #[test]
    fn every_probe_name_made_is_one_the_sweep_knows() {
        for kind in Probe::ALL {
            let tag = new_tag();
            assert_eq!(kind_of(&kind.name(tag)), Some(kind));
        }
        assert_ne!(new_tag(), new_tag());
    }

    /// Each kind holds only its own bytes: a time probe's name with a drive probe's bytes, or the
    /// other way round, is not a probe file.
    #[test]
    fn each_kind_holds_only_its_own_bytes() {
        assert!(Probe::DriveC.holds(DRIVE_A) && Probe::DriveC.holds(DRIVE_B));
        assert!(!Probe::DriveA.holds(TIME));
        assert!(Probe::TimeU.holds(TIME) && !Probe::TimeU.holds(DRIVE_A));
        assert!(Probe::ReadBack.holds(&read_back_bytes()));
        assert!(!Probe::ReadBack.holds(TIME));
        assert!(!Probe::TimeT.holds(b""));
    }
}
