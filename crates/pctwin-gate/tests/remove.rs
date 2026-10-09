//! Undo removes only the file it checked (Security Design B, decided 8 October 2026; Task List
//! 2.3). The gate opens the copy relative to its approved folder and checks on that same handle
//! that it is a regular file, the very file PCTwin wrote, with one name, and unchanged (the
//! caller's check, read through the handle). On Windows it removes that handle's file; on Linux
//! and macOS it moves the name aside first and checks again there (decided 9 October 2026; the
//! steps of that are tested in `unix_undo.rs`). Nothing is acted on by name after a check. These
//! tests never use the person's real Recycle Bin.

use std::io::Read;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(windows)]
use pctwin_gate::Left;
use pctwin_gate::{Destination, FileId, GateError, Removed};

/// One undo journal for the whole test run, open: every removal needs one of its permits.
fn journal() -> &'static pctwin_journal::Journal {
    static J: std::sync::OnceLock<(tempfile::TempDir, pctwin_journal::Journal)> =
        std::sync::OnceLock::new();
    &J.get_or_init(|| {
        let d = tempfile::tempdir().unwrap();
        let j = pctwin_journal::Journal::open(&d.path().join("j.redb")).unwrap();
        (d, j)
    })
    .1
}

/// Removal as undo does it, with nothing to record first.
trait Rm {
    fn rm(
        &self,
        stored: &str,
        expect: FileId,
        verify: impl FnOnce(&mut std::fs::File) -> std::io::Result<bool>,
    ) -> Result<Removed, GateError>;
}

impl Rm for Destination {
    fn rm(
        &self,
        stored: &str,
        expect: FileId,
        verify: impl FnOnce(&mut std::fs::File) -> std::io::Result<bool>,
    ) -> Result<Removed, GateError> {
        let permit = journal().begin_undo().unwrap();
        #[cfg(windows)]
        return self.remove_if_unchanged(&permit, stored, expect, verify, || Ok(()));
        #[cfg(unix)]
        {
            let copy = match self.check_copy(stored, expect, verify)? {
                pctwin_gate::Check::Ready(copy) => copy,
                pctwin_gate::Check::Done(r) => return Ok(r),
            };
            let private = pctwin_gate::private_name()?;
            let mut nothing = |_: pctwin_gate::Step<'_>| Ok(());
            let mut cx = pctwin_gate::Context {
                kept_words: " (kept by PCTwin undo)",
                room_for_words: 64,
                others_closed: false,
                journal: &mut nothing,
            };
            self.remove_checked(&permit, *copy, &private, &mut cx)
        }
    }
}

fn setup() -> (tempfile::TempDir, Destination) {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("Docs")).unwrap();
    let dest = Destination::open(root.path()).unwrap();
    (root, dest)
}

/// The copy's identity as PCTwin records it. PCTwin keeps each file it makes open until 10 ms
/// after it was made, so nothing made later shares its number and birth time; the wait here
/// stands in for that.
fn id(dest: &Destination, stored: &str) -> FileId {
    std::thread::sleep(std::time::Duration::from_millis(11));
    dest.stat(stored).unwrap().unwrap().id
}

fn put(root: &tempfile::TempDir, stored: &str, bytes: &[u8]) {
    std::fs::write(root.path().join(stored), bytes).unwrap();
}

fn names(dir: &std::path::Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

/// The caller's check: the bytes read through the held handle are exactly `want`.
fn bytes_are(want: &'static [u8]) -> impl FnOnce(&mut std::fs::File) -> std::io::Result<bool> {
    move |f| {
        let mut got = Vec::new();
        f.read_to_end(&mut got)?;
        Ok(got == want)
    }
}

#[test]
fn the_very_file_unchanged_is_removed() {
    let (root, dest) = setup();
    put(&root, "Docs/a.txt", b"copy");
    let file = id(&dest, "Docs/a.txt");
    let r = dest.rm("Docs/a.txt", file, bytes_are(b"copy")).unwrap();
    assert_eq!(r, Removed::Removed);
    assert!(names(&root.path().join("Docs")).is_empty());
}

#[test]
fn another_file_at_the_name_is_never_removed() {
    let (root, dest) = setup();
    put(&root, "Docs/a.txt", b"copy");
    let file = id(&dest, "Docs/a.txt");
    // The person's own file now has the name (the copy was removed and another made).
    std::fs::remove_file(root.path().join("Docs/a.txt")).unwrap();
    put(&root, "Docs/other.txt", b"keep");
    put(&root, "Docs/a.txt", b"copy");
    let mut called = false;
    let r = dest
        .rm("Docs/a.txt", file, |_| {
            called = true;
            Ok(true)
        })
        .unwrap();
    assert_eq!(r, Removed::NotThatFile);
    assert!(!called, "the caller's check runs only on the very file");
    assert_eq!(
        std::fs::read(root.path().join("Docs/a.txt")).unwrap(),
        b"copy"
    );
}

#[test]
fn a_file_changed_since_is_kept() {
    let (root, dest) = setup();
    put(&root, "Docs/a.txt", b"copy");
    let file = id(&dest, "Docs/a.txt");
    let r = dest
        .rm("Docs/a.txt", file, bytes_are(b"something else"))
        .unwrap();
    assert_eq!(r, Removed::Changed);
    assert_eq!(
        std::fs::read(root.path().join("Docs/a.txt")).unwrap(),
        b"copy"
    );
    assert_eq!(names(&root.path().join("Docs")), ["a.txt"]);
}

#[test]
fn a_check_that_cannot_finish_removes_nothing() {
    let (root, dest) = setup();
    put(&root, "Docs/a.txt", b"copy");
    let file = id(&dest, "Docs/a.txt");
    let r = dest.rm("Docs/a.txt", file, |_| {
        Err(std::io::Error::other("the drive stopped answering"))
    });
    assert!(r.is_err());
    assert_eq!(
        std::fs::read(root.path().join("Docs/a.txt")).unwrap(),
        b"copy"
    );
}

#[test]
fn a_file_no_longer_there_is_gone() {
    let (root, dest) = setup();
    put(&root, "Docs/a.txt", b"copy");
    let file = id(&dest, "Docs/a.txt");
    std::fs::remove_file(root.path().join("Docs/a.txt")).unwrap();
    for stored in ["Docs/a.txt", "Missing/a.txt"] {
        let r = dest
            .rm(stored, file, |_| panic!("nothing to check"))
            .unwrap();
        assert_eq!(r, Removed::Gone, "{stored}");
    }
}

#[test]
fn a_file_with_a_second_name_is_kept() {
    let (root, dest) = setup();
    put(&root, "Docs/a.txt", b"copy");
    let file = id(&dest, "Docs/a.txt");
    std::fs::hard_link(
        root.path().join("Docs/a.txt"),
        root.path().join("Docs/second.txt"),
    )
    .unwrap();
    let r = dest.rm("Docs/a.txt", file, |_| Ok(true)).unwrap();
    assert_eq!(r, Removed::Linked);
    assert_eq!(names(&root.path().join("Docs")), ["a.txt", "second.txt"]);
}

#[test]
fn a_folder_at_the_name_is_never_removed() {
    let (root, dest) = setup();
    std::fs::create_dir(root.path().join("Docs/a.txt")).unwrap();
    let folder = dest.folder_identity("Docs/a.txt").unwrap().unwrap();
    let r = dest.rm("Docs/a.txt", folder, |_| Ok(true)).unwrap();
    assert_eq!(r, Removed::NotThatFile);
    assert!(root.path().join("Docs/a.txt").is_dir());
}

#[test]
fn a_link_at_the_name_is_never_followed_or_removed() {
    let (root, dest) = setup();
    put(&root, "Docs/target.txt", b"the person's");
    let target = id(&dest, "Docs/target.txt");
    #[cfg(unix)]
    let made = std::os::unix::fs::symlink(
        root.path().join("Docs/target.txt"),
        root.path().join("Docs/a.txt"),
    );
    #[cfg(windows)]
    let made = std::os::windows::fs::symlink_file(
        root.path().join("Docs/target.txt"),
        root.path().join("Docs/a.txt"),
    );
    if made.is_err() {
        // Windows without the right to make links (no developer mode): CI covers it.
        eprintln!("skipped: cannot make a link here");
        return;
    }
    // Even told the link's target is the file, the link is never followed.
    let r = dest.rm("Docs/a.txt", target, |_| Ok(true)).unwrap();
    assert_eq!(r, Removed::NotThatFile);
    assert_eq!(
        std::fs::read(root.path().join("Docs/target.txt")).unwrap(),
        b"the person's"
    );
    assert!(
        std::fs::symlink_metadata(root.path().join("Docs/a.txt"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
}

#[test]
fn a_read_only_copy_is_removed() {
    let (root, dest) = setup();
    put(&root, "Docs/a.txt", b"copy");
    let p = root.path().join("Docs/a.txt");
    let mut perms = std::fs::metadata(&p).unwrap().permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(&p, perms).unwrap();
    let file = id(&dest, "Docs/a.txt");
    let r = dest.rm("Docs/a.txt", file, bytes_are(b"copy")).unwrap();
    assert_eq!(r, Removed::Removed);
    assert!(!p.exists());
}

#[test]
fn only_the_one_name_is_touched_and_no_private_name_is_left() {
    let (root, dest) = setup();
    put(&root, "Docs/a.txt", b"copy");
    put(&root, "Docs/b.txt", b"mine");
    let file = id(&dest, "Docs/a.txt");
    let r = dest.rm("Docs/a.txt", file, bytes_are(b"copy")).unwrap();
    assert_eq!(r, Removed::Removed);
    assert_eq!(names(&root.path().join("Docs")), ["b.txt"]);
}

/// Several tries at once: each copy removed exactly once, the rest refused, never an error.
#[test]
fn removing_the_same_file_twice_at_once_removes_it_once() {
    let (root, dest) = setup();
    put(&root, "Docs/a.txt", b"copy");
    let file = id(&dest, "Docs/a.txt");
    let dest = Arc::new(dest);
    let results: Vec<Removed> = std::thread::scope(|s| {
        let tries: Vec<_> = (0..4)
            .map(|_| {
                let dest = Arc::clone(&dest);
                s.spawn(move || dest.rm("Docs/a.txt", file, bytes_are(b"copy")).unwrap())
            })
            .collect();
        tries.into_iter().map(|t| t.join().unwrap()).collect()
    });
    let removed = results.iter().filter(|r| **r == Removed::Removed).count();
    assert!(
        results
            .iter()
            .all(|r| matches!(r, Removed::Removed | Removed::Gone | Removed::InUse)),
        "{results:?}"
    );
    // Windows waits out a handle held for a moment, so one try always removes it. On Linux each
    // try sees the others' handles open, so all may correctly say "in use": then it is all there.
    if cfg!(windows) || removed > 0 {
        assert_eq!(removed, 1, "{results:?}");
        assert!(names(&root.path().join("Docs")).is_empty());
    } else {
        assert_eq!(
            std::fs::read(root.path().join("Docs/a.txt")).unwrap(),
            b"copy"
        );
    }
}

/// The race the redesign is for: while undo checks and removes, the person saves a new version of
/// the file by writing another and putting it over the name (as editors do). Over many tries, the
/// person's new version is never lost, whatever undo reports.
#[test]
fn a_new_version_saved_over_the_name_during_undo_is_never_lost() {
    let trials = if std::env::var_os("CI").is_some() {
        1000
    } else {
        300
    };
    let (root, dest) = setup();
    let docs = root.path().join("Docs");
    let mut removed = 0;
    for n in 0..trials {
        put(&root, "Docs/a.txt", b"copy");
        let file = id(&dest, "Docs/a.txt");
        std::fs::write(docs.join("new.tmp"), format!("edit {n}")).unwrap();
        let go = Arc::new(AtomicBool::new(false));
        let r = std::thread::scope(|s| {
            let go2 = Arc::clone(&go);
            let docs2 = docs.clone();
            let saver = s.spawn(move || {
                while !go2.load(Ordering::Acquire) {
                    std::hint::spin_loop();
                }
                // Keep trying until the save lands (a held file refuses it for a moment).
                for _ in 0..100_000 {
                    if std::fs::rename(docs2.join("new.tmp"), docs2.join("a.txt")).is_ok() {
                        return;
                    }
                    std::thread::yield_now();
                }
                panic!("the save never landed");
            });
            go.store(true, Ordering::Release);
            let r = dest.rm("Docs/a.txt", file, bytes_are(b"copy"));
            saver.join().unwrap();
            r
        });
        let r = r.unwrap();
        if r == Removed::Removed {
            removed += 1;
        }
        // The person's new version is there, whatever happened.
        assert_eq!(
            std::fs::read_to_string(docs.join("a.txt")).unwrap(),
            format!("edit {n}"),
            "trial {n}: {r:?}"
        );
        assert_eq!(names(&docs), ["a.txt"], "trial {n}: {r:?}");
        std::fs::remove_file(docs.join("a.txt")).unwrap();
    }
    eprintln!("removed before the save landed in {removed} of {trials} trials");
}

/// A look-alike (same bytes, another file) swapped in at the name during undo is never removed.
#[test]
fn a_look_alike_swapped_in_during_undo_is_never_removed() {
    let trials = if std::env::var_os("CI").is_some() {
        1000
    } else {
        300
    };
    let (root, dest) = setup();
    let docs = root.path().join("Docs");
    for n in 0..trials {
        put(&root, "Docs/a.txt", b"copy");
        let file = id(&dest, "Docs/a.txt");
        put(&root, "Docs/look.tmp", b"copy");
        let look = id(&dest, "Docs/look.tmp");
        let go = Arc::new(AtomicBool::new(false));
        let r = std::thread::scope(|s| {
            let go2 = Arc::clone(&go);
            let docs2 = docs.clone();
            let swapper = s.spawn(move || {
                while !go2.load(Ordering::Acquire) {
                    std::hint::spin_loop();
                }
                for _ in 0..100_000 {
                    if std::fs::rename(docs2.join("look.tmp"), docs2.join("a.txt")).is_ok() {
                        return;
                    }
                    std::thread::yield_now();
                }
                panic!("the swap never landed");
            });
            go.store(true, Ordering::Release);
            let r = dest.rm("Docs/a.txt", file, bytes_are(b"copy"));
            swapper.join().unwrap();
            r
        });
        let r = r.unwrap();
        // The look-alike is at the name, untouched, whatever undo reported.
        assert_eq!(id(&dest, "Docs/a.txt"), look, "trial {n}: {r:?}");
        assert_eq!(names(&docs), ["a.txt"], "trial {n}: {r:?}");
        std::fs::remove_file(docs.join("a.txt")).unwrap();
    }
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::os::windows::fs::OpenOptionsExt;

    const SHARE_READ: u32 = 1;
    const SHARE_WRITE: u32 = 2;
    const SHARE_DELETE: u32 = 4;

    /// While the check runs on the held handle, nobody can write, rename, replace or remove the
    /// file; readers that allow removal (as antivirus does) still can read.
    #[test]
    fn while_the_file_is_checked_nobody_can_change_move_replace_or_remove_it() {
        let (root, dest) = setup();
        put(&root, "Docs/a.txt", b"copy");
        put(&root, "Docs/other.txt", b"other");
        let p = root.path().join("Docs/a.txt");
        let other = root.path().join("Docs/other.txt");
        let file = id(&dest, "Docs/a.txt");
        let r = dest
            .rm("Docs/a.txt", file, |f| {
                // Writing, with or without sharing.
                assert!(std::fs::OpenOptions::new().write(true).open(&p).is_err());
                assert!(
                    std::fs::OpenOptions::new()
                        .write(true)
                        .share_mode(SHARE_READ | SHARE_WRITE | SHARE_DELETE)
                        .open(&p)
                        .is_err()
                );
                // Moving it away, putting another over it, removing it.
                assert!(std::fs::rename(&p, root.path().join("Docs/moved.txt")).is_err());
                assert!(std::fs::rename(&other, &p).is_err());
                assert!(std::fs::remove_file(&p).is_err());
                // A reader that allows removal still reads.
                let mut reader = std::fs::OpenOptions::new()
                    .read(true)
                    .share_mode(SHARE_READ | SHARE_WRITE | SHARE_DELETE)
                    .open(&p)
                    .unwrap();
                let mut seen = Vec::new();
                reader.read_to_end(&mut seen).unwrap();
                assert_eq!(seen, b"copy");
                bytes_are(b"copy")(f)
            })
            .unwrap();
        assert_eq!(r, Removed::Removed);
        assert_eq!(names(&root.path().join("Docs")), ["other.txt"]);
    }

    #[test]
    fn a_file_another_program_is_writing_is_kept_as_in_use() {
        let (root, dest) = setup();
        put(&root, "Docs/a.txt", b"copy");
        let file = id(&dest, "Docs/a.txt");
        let _writer = std::fs::OpenOptions::new()
            .write(true)
            .open(root.path().join("Docs/a.txt"))
            .unwrap();
        let r = dest
            .rm("Docs/a.txt", file, |_| panic!("never checked while in use"))
            .unwrap();
        assert_eq!(r, Removed::InUse);
        assert!(root.path().join("Docs/a.txt").exists());
    }

    /// A program that has the file open for a moment (a quick save, an indexer) does not stop
    /// undo: it waits a little and tries again.
    #[test]
    fn a_file_held_for_a_moment_is_still_removed() {
        let (root, dest) = setup();
        put(&root, "Docs/a.txt", b"copy");
        let file = id(&dest, "Docs/a.txt");
        let writer = std::fs::OpenOptions::new()
            .write(true)
            .open(root.path().join("Docs/a.txt"))
            .unwrap();
        let r = std::thread::scope(|s| {
            s.spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(30));
                drop(writer);
            });
            dest.rm("Docs/a.txt", file, bytes_are(b"copy")).unwrap()
        });
        assert_eq!(r, Removed::Removed);
    }

    #[test]
    fn a_reader_that_does_not_allow_removal_keeps_the_file_in_use() {
        let (root, dest) = setup();
        put(&root, "Docs/a.txt", b"copy");
        let file = id(&dest, "Docs/a.txt");
        let _reader = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(SHARE_READ | SHARE_WRITE)
            .open(root.path().join("Docs/a.txt"))
            .unwrap();
        let r = dest.rm("Docs/a.txt", file, |_| Ok(true)).unwrap();
        assert_eq!(r, Removed::InUse);
        assert!(root.path().join("Docs/a.txt").exists());
    }

    #[test]
    fn a_reader_that_allows_removal_does_not_stop_it_and_the_name_is_free_at_once() {
        let (root, dest) = setup();
        put(&root, "Docs/a.txt", b"copy");
        let file = id(&dest, "Docs/a.txt");
        let mut reader = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(SHARE_READ | SHARE_WRITE | SHARE_DELETE)
            .open(root.path().join("Docs/a.txt"))
            .unwrap();
        let r = dest.rm("Docs/a.txt", file, bytes_are(b"copy")).unwrap();
        assert_eq!(r, Removed::Removed);
        // The name is free while the reader still reads the old bytes.
        assert!(!root.path().join("Docs/a.txt").exists());
        std::fs::write(root.path().join("Docs/a.txt"), b"new").unwrap();
        let mut seen = Vec::new();
        reader.read_to_end(&mut seen).unwrap();
        assert_eq!(seen, b"copy");
    }

    /// A file stored online only (here marked offline, as such files are) is never opened for its
    /// bytes, so it is never downloaded, and never removed.
    #[test]
    fn a_file_stored_online_only_is_kept() {
        let (root, dest) = setup();
        put(&root, "Docs/a.txt", b"copy");
        let file = id(&dest, "Docs/a.txt");
        let p = root.path().join("Docs/a.txt");
        let ok = std::process::Command::new("attrib")
            .arg("+O")
            .arg(&p)
            .status()
            .is_ok_and(|s| s.success());
        assert!(ok, "attrib +O failed");
        let r = dest
            .rm("Docs/a.txt", file, |_| panic!("never read"))
            .unwrap();
        assert_eq!(r, Removed::CloudOnly);
        assert!(p.exists());
    }
}

#[cfg(windows)]
#[test]
fn a_junction_at_the_name_is_never_followed_or_removed() {
    let (root, dest) = setup();
    std::fs::create_dir(root.path().join("Elsewhere")).unwrap();
    put(&root, "Elsewhere/keep.txt", b"the person's");
    let ok = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(root.path().join("Docs").join("a.txt"))
        .arg(root.path().join("Elsewhere"))
        .output()
        .is_ok_and(|o| o.status.success());
    assert!(ok, "mklink /J failed");
    let folder = dest.folder_identity("Elsewhere").unwrap().unwrap();
    let r = dest.rm("Docs/a.txt", folder, |_| Ok(true)).unwrap();
    assert_eq!(r, Removed::NotThatFile);
    assert_eq!(
        std::fs::read(root.path().join("Elsewhere/keep.txt")).unwrap(),
        b"the person's"
    );
}

/// PCTwin's own temporary and private names are never names undo removes through this.
#[test]
fn pctwin_s_own_names_are_refused() {
    let (root, dest) = setup();
    for name in [
        pctwin_gate::temp_name("ab-1"),
        ".pctwin-undo-1-2".to_string(),
        format!(".pctwin-undo-{}", "a".repeat(32)),
        format!(".pctwin-salvage-{}", "b".repeat(32)),
    ] {
        put(&root, &format!("Docs/{name}"), b"x");
        let file = id(&dest, &format!("Docs/{name}"));
        let r = dest.rm(&format!("Docs/{name}"), file, |_| Ok(true));
        assert!(r.is_err(), "{name}: {r:?}");
        assert!(root.path().join("Docs").join(&name).exists());
    }
}

/// Sharing that blocks writers does not block making another name: one made while the file is
/// checked keeps it (the number of names is read again just before the removal).
#[test]
fn a_second_name_made_while_the_file_is_checked_keeps_it() {
    let (root, dest) = setup();
    put(&root, "Docs/a.txt", b"copy");
    let file = id(&dest, "Docs/a.txt");
    let (p, q) = (
        root.path().join("Docs/a.txt"),
        root.path().join("Docs/theirs.txt"),
    );
    let r = dest
        .rm("Docs/a.txt", file, |f| {
            std::fs::hard_link(&p, &q).unwrap();
            bytes_are(b"copy")(f)
        })
        .unwrap();
    assert_eq!(r, Removed::Linked);
    assert_eq!(names(&root.path().join("Docs")), ["a.txt", "theirs.txt"]);
}

/// On Linux and macOS a file recorded without a birth time cannot be told from one made later in
/// its freed number: never removed. (A file number of 0 cannot even be recorded.)
#[cfg(unix)]
#[test]
fn a_file_with_no_birth_time_is_never_removed() {
    let (root, dest) = setup();
    put(&root, "Docs/a.txt", b"copy");
    let mut file = id(&dest, "Docs/a.txt");
    file.born = None;
    let r = dest.rm("Docs/a.txt", file, |_| Ok(true)).unwrap();
    assert_eq!(r, Removed::Unsupported);
    assert!(root.path().join("Docs/a.txt").exists());
}

/// The last call before the removal comes once, only when the file is really being removed, and
/// if it fails nothing is removed. (Linux and macOS record each step instead: `unix_undo.rs`.)
#[cfg(windows)]
#[test]
fn the_removal_is_announced_once_only_when_it_happens() {
    let (root, dest) = setup();
    put(&root, "Docs/a.txt", b"copy");
    let file = id(&dest, "Docs/a.txt");
    let permit = journal().begin_undo().unwrap();
    // Changed: never announced.
    let mut told = 0;
    let r = dest
        .remove_if_unchanged(&permit, "Docs/a.txt", file, bytes_are(b"other"), || {
            told += 1;
            Ok(())
        })
        .unwrap();
    assert_eq!((r, told), (Removed::Changed, 0));
    // The announcement fails: nothing is removed.
    let r = dest.remove_if_unchanged(&permit, "Docs/a.txt", file, bytes_are(b"copy"), || {
        Err(std::io::Error::other("the journal could not be written"))
    });
    assert!(r.is_err());
    assert!(root.path().join("Docs/a.txt").exists());
    // Removed: announced once, before.
    let r = dest
        .remove_if_unchanged(&permit, "Docs/a.txt", file, bytes_are(b"copy"), || {
            told += 1;
            assert!(root.path().join("Docs/a.txt").exists(), "announced first");
            Ok(())
        })
        .unwrap();
    assert_eq!((r, told), (Removed::Removed, 1));
}

#[cfg(windows)]
#[test]
fn a_removal_under_way_is_resumed_from_what_is_on_the_disk() {
    let (root, dest) = setup();
    put(&root, "Docs/a.txt", b"copy");
    let file = id(&dest, "Docs/a.txt");
    let permit = journal().begin_undo().unwrap();
    assert_eq!(
        dest.resume_removal(&permit, "Docs/a.txt", file).unwrap(),
        Left::Here
    );
    // Another file under its name now: the copy is gone.
    std::fs::remove_file(root.path().join("Docs/a.txt")).unwrap();
    assert_eq!(
        dest.resume_removal(&permit, "Docs/a.txt", file).unwrap(),
        Left::Gone
    );
    put(&root, "Docs/a.txt", b"copy");
    assert_eq!(
        dest.resume_removal(&permit, "Docs/a.txt", file).unwrap(),
        Left::Gone
    );
    assert_eq!(
        dest.resume_removal(&permit, "Nowhere/a.txt", file).unwrap(),
        Left::Gone
    );
}
