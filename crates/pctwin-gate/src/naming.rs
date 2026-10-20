//! How a drive gives a finished file its real name without ever replacing anything, found once
//! per drive by trying it on PCTwin's own probe files (security design notes, "Naming a finished
//! file never replaces anything" and "Drives with no safe rename", decided 15 October 2026).
//!
//! The test, in a folder of the drive:
//! 1. Probe files a crash left behind (of every test: see the probes module) are removed, but
//!    only those holding a probe's own bytes and older than a test takes (a test running now,
//!    from another handle on the drive, is never disturbed); anything else stays.
//! 2. Two probe files `a` and `b` are made, each only if nothing has its name; making `a` again
//!    must then be refused (exclusive create works).
//! 3. A hard link from `a` to a new name `c`, and from `a` onto `b`, which must be refused.
//! 4. A no-replace move of `a` onto `b`: refused because `b` exists ("exists"), with `b` intact,
//!    is half the answer; refused as unsupported ("invalid", "not implemented", "not supported")
//!    means it has no such move; done, with `b` silently replaced, means it ignores the request,
//!    which is recorded ([`DriveCheck::replaced_silently`]) for the report.
//! 5. Then the same move of `a` onto the free name `c`, which must simply work. Both halves are
//!    needed: macOS's exFAT driver answers "exists" for a name that is taken before it answers
//!    "not supported" for the move itself, so "exists" alone proves nothing (found on real
//!    drives in CI, 15 October 2026).
//! 6. All probe files are removed.
//!
//! A hard link or an honoured no-replace move makes the whole file appear at once
//! ([`Naming::Atomic`]). Without either, the name is created only if free and then filled
//! ([`Naming::FillIn`]); a drive where even an exclusive create is not honoured has no safe way
//! at all ([`Naming::NoSafeMethod`]) and is refused when it is chosen. Each error number is read
//! only against this result: an "invalid" from a drive whose test passed is the name itself.

use std::io::{self, Read, Write};

use crate::probes::{self, Probe};

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
    /// A creation time set on a file is kept exactly (the tag that proves a file filled in under
    /// its real name is PCTwin's, where identities change).
    pub exact_birth_time: bool,
    /// A file can be marked hidden through its handle (a file filled in under its real name is
    /// hidden until it is complete).
    pub hidden: bool,
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
            exact_birth_time: crate::fill::CAN_SET_BORN,
            hidden: crate::fill::CAN_HIDE,
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
    moved_to_free_name: bool,
    exclusive_create: bool,
) -> DriveCheck {
    let hard_links = link_new && link_onto_existing_refused;
    let no_replace_move = matches!(moved, Moved::Taken) && b_intact && moved_to_free_name;
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
        exact_birth_time: false,
        hidden: false,
    }
}

/// A drive that must fill files in may do so only if such a file can be hidden until it is
/// complete and proven PCTwin's after a restart (a creation time it keeps exactly, or identities
/// it keeps for good); else it is refused when it is chosen (security design notes, "Drives with
/// no safe rename": "A drive where even this cannot be done safely is refused").
pub(crate) fn provable(
    check: DriveCheck,
    kind: crate::DriveKind,
    system: crate::Platform,
) -> DriveCheck {
    if check.naming != Naming::FillIn {
        return check;
    }
    let lasting = check.exact_birth_time
        || kind.keeps_identities()
        || (system == crate::Platform::Windows && check.hard_links);
    if lasting && check.hidden {
        check
    } else {
        DriveCheck {
            naming: Naming::NoSafeMethod,
            ..check
        }
    }
}

/// What a drive's test may trust on `system`, for a drive of `kind`. On a Mac a no-replace move
/// counts only on APFS and HFS+, which do it themselves: FAT32 and exFAT say they cannot
/// (`VOL_CAP_INT_RENAME_EXCL` is 0, found in CI on 15 October 2026), so macOS checks the name
/// itself before it moves, and that check and the move are two steps another program could come
/// between. Those drives fill files in instead, where the drive itself refuses a taken name.
pub(crate) fn for_system(
    check: DriveCheck,
    kind: crate::DriveKind,
    system: crate::Platform,
) -> DriveCheck {
    use crate::DriveKind::{Apfs, HfsPlus};
    if system != crate::Platform::MacOs || check.hard_links || matches!(kind, Apfs | HfsPlus) {
        return check;
    }
    let naming = match check.naming {
        Naming::Atomic if check.no_replace_move => Naming::FillIn,
        n => n,
    };
    DriveCheck {
        naming,
        no_replace_move: false,
        ..check
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
    probes::sweep(dir)?;
    let tag = probes::new_tag();
    let [a, b, c] = [Probe::DriveA, Probe::DriveB, Probe::DriveC].map(|p| p.name(tag));
    let result = run(dir, [&a, &b, &c], folder_path, move_no_replace);
    // Every probe file goes, whatever the test found.
    let mut cleared = Ok(());
    for name in [&a, &b, &c] {
        if let Err(e) = probes::remove_if_probe(dir, name, false) {
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
    make(dir, a, probes::DRIVE_A)?;
    let exact_birth_time = crate::fill::keeps_born(dir, a);
    let hidden = crate::fill::hides(dir, a);
    make(dir, b, probes::DRIVE_B)?;
    let exclusive_create = match make(dir, a, probes::DRIVE_B) {
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => true,
        Err(e) => return Err(e),
        // Made over an existing file: exclusive create is not honoured. Put `a` back as it was.
        Ok(()) => {
            probes::remove_if_probe(dir, a, false)?;
            make(dir, a, probes::DRIVE_A)?;
            false
        }
    };
    let link_new = dir.hard_link(a, dir, c).is_ok();
    let link_onto_existing_refused = link_new
        && matches!(dir.hard_link(a, dir, b), Err(e) if e.kind() == io::ErrorKind::AlreadyExists);
    if link_new {
        // The link is another name for `a`'s bytes: removed with the rest, the same careful way.
        probes::remove_if_probe(dir, c, false)?;
    }
    let moved = move_no_replace(dir, a, b, folder_path);
    if let Moved::Failed(e) = moved {
        return Err(e);
    }
    let mut now = Vec::new();
    dir.open(b)?.read_to_end(&mut now)?;
    // Refused onto a taken name: does the same move onto a free name work at all? (`c` is free:
    // the hard link test made it only for a moment.)
    let moved_to_free_name = match &moved {
        Moved::Taken => match move_no_replace(dir, a, c, folder_path) {
            Moved::Done => true,
            Moved::Failed(e) => return Err(e),
            _ => false,
        },
        _ => false,
    };
    Ok(DriveCheck {
        exact_birth_time,
        hidden,
        ..classify(
            link_new,
            link_onto_existing_refused,
            &moved,
            now == probes::DRIVE_B,
            moved_to_free_name,
            exclusive_create,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_each_answer_means() {
        use Naming::*;
        let err = || Moved::Failed(io::Error::other("x"));
        // (link to a new name, link onto b refused, the move onto b, b intact, the move onto a
        // free name worked, exclusive create)
        for (link_new, refused, moved, intact, free, excl, naming, hard, mv, silent) in [
            // NTFS, ext4, APFS: hard links.
            (
                true,
                true,
                Moved::Taken,
                true,
                true,
                true,
                Atomic,
                true,
                true,
                false,
            ),
            // FAT, exFAT on Linux and Windows; FAT32 on a Mac: only the no-replace move.
            (
                false,
                false,
                Moved::Taken,
                true,
                true,
                true,
                Atomic,
                false,
                true,
                false,
            ),
            // exFAT on a Mac: "exists" for a taken name, yet "not supported" onto a free one.
            (
                false,
                false,
                Moved::Taken,
                true,
                false,
                true,
                FillIn,
                false,
                false,
                false,
            ),
            (
                false,
                false,
                Moved::Invalid,
                true,
                false,
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
                false,
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
                false,
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
                false,
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
                false,
                true,
                FillIn,
                false,
                false,
                false,
            ),
        ] {
            let c = classify(link_new, refused, &moved, intact, free, excl);
            assert_eq!(
                (
                    c.naming,
                    c.hard_links,
                    c.no_replace_move,
                    c.replaced_silently
                ),
                (naming, hard, mv, silent),
                "{link_new} {refused} {moved:?} {intact} {free} {excl}"
            );
            assert_eq!(c.exclusive_create, excl);
        }
        // A step that failed for another reason is no answer at all (the test is run again).
        assert!(!classify(false, false, &err(), true, true, true).no_replace_move);
    }
}

#[cfg(test)]
mod trust_tests {
    use super::*;
    use crate::{DriveKind, Platform};

    fn moving() -> DriveCheck {
        classify(false, false, &Moved::Taken, true, true, true)
    }

    #[test]
    fn a_mac_trusts_a_no_replace_move_only_where_the_drive_does_it_itself() {
        for kind in [
            DriveKind::Fat,
            DriveKind::ExFat,
            DriveKind::Unknown,
            DriveKind::Network,
        ] {
            let c = for_system(moving(), kind, Platform::MacOs);
            assert_eq!(c.naming, Naming::FillIn, "{kind:?}");
            assert!(!c.no_replace_move, "{kind:?}");
        }
        for kind in [DriveKind::Apfs, DriveKind::HfsPlus] {
            assert_eq!(
                for_system(moving(), kind, Platform::MacOs),
                moving(),
                "{kind:?}"
            );
        }
        // Linux and Windows do the no-replace move in the drive driver itself.
        for system in [Platform::Linux, Platform::Windows] {
            assert_eq!(for_system(moving(), DriveKind::Fat, system), moving());
        }
        // A drive with hard links is unchanged; one with no safe way stays refused.
        let linked = classify(true, true, &Moved::Taken, true, true, true);
        assert_eq!(for_system(linked, DriveKind::Fat, Platform::MacOs), linked);
        let none = classify(false, false, &Moved::Taken, true, true, false);
        assert_eq!(
            for_system(none, DriveKind::Fat, Platform::MacOs).naming,
            Naming::NoSafeMethod
        );
    }
}

#[cfg(test)]
mod provable_tests {
    use super::*;
    use crate::{DriveKind, Platform};

    fn fill_in(birth: bool, hidden: bool, hard_links: bool) -> DriveCheck {
        DriveCheck {
            naming: Naming::FillIn,
            hard_links,
            no_replace_move: false,
            replaced_silently: false,
            exclusive_create: true,
            exact_birth_time: birth,
            hidden,
        }
    }

    #[test]
    fn a_drive_is_filled_in_only_if_a_file_can_be_hidden_and_proven_after_a_restart() {
        let mac = Platform::MacOs;
        // exFAT on a Mac: tag and hidden mark both kept.
        assert_eq!(
            provable(fill_in(true, true, false), DriveKind::ExFat, mac).naming,
            Naming::FillIn
        );
        // No hidden mark: refused.
        assert_eq!(
            provable(fill_in(true, false, false), DriveKind::ExFat, mac).naming,
            Naming::NoSafeMethod
        );
        // Hidden but nothing lasting (no tag kept, identities change): refused.
        assert_eq!(
            provable(fill_in(false, true, false), DriveKind::ExFat, mac).naming,
            Naming::NoSafeMethod
        );
        // Identities kept for good stand in for the tag.
        assert_eq!(
            provable(fill_in(false, true, false), DriveKind::Apfs, mac).naming,
            Naming::FillIn
        );
        // On Windows, hard links mean identities that last.
        assert_eq!(
            provable(
                fill_in(false, true, true),
                DriveKind::Unknown,
                Platform::Windows
            )
            .naming,
            Naming::FillIn
        );
        // A drive FUSE serves on Linux: no tag, no hidden mark, identities unknown: refused.
        assert_eq!(
            provable(
                fill_in(false, false, false),
                DriveKind::Unknown,
                Platform::Linux
            )
            .naming,
            Naming::NoSafeMethod
        );
        // Drives that name at once are not touched.
        let atomic = classify(true, true, &Moved::Taken, true, true, true);
        assert_eq!(
            provable(atomic, DriveKind::Unknown, Platform::Linux),
            atomic
        );
    }
}
