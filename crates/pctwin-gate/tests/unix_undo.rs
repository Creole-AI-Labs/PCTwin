//! Undo's removal on Linux and macOS: move aside first, then check (Security Design 3B, undo
//! bullet, decided 9 October 2026). The copy is checked through a held handle, its name moved,
//! never replacing, to a random private name the journal recorded first, proven there to be the
//! very file unchanged, and only then removed. Anything else goes back under its name or beside
//! it; nothing is lost and nothing is left hidden. A part-way removal is finished from the disk.
//!
//! These run on drives undo removes from (ext4, XFS, btrfs, ZFS, F2FS; APFS). Elsewhere (Docker's
//! own file system, tmpfs) they say so and pass, unless PCTWIN_REQUIRE_UNIX_UNDO is set, as it is
//! on CI and in the Linux drive rig, where they must run.
#![cfg(unix)]

use std::io::Read;
use std::path::Path;

use pctwin_gate::{Check, Context, Destination, FileId, Removed, Resolution, Step};
use pctwin_journal::Journal;

const WORDS: &str = " (kept by PCTwin undo)";

fn journal() -> &'static Journal {
    let dir = Box::leak(Box::new(tempfile::tempdir().unwrap()));
    Box::leak(Box::new(
        Journal::open(&dir.path().join("journal.redb")).unwrap(),
    ))
}

/// A destination on a drive undo removes from, or `None` (said, and allowed only when not
/// required).
fn setup() -> Option<(tempfile::TempDir, Destination)> {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("Docs")).unwrap();
    let dest = Destination::open(root.path()).unwrap();
    put(&root, "Docs/probe.txt", b"probe");
    let file = id(&dest, "Docs/probe.txt");
    let ready = matches!(
        dest.check_copy("Docs/probe.txt", file, |_| Ok(true))
            .unwrap(),
        Check::Ready(_)
    );
    std::fs::remove_file(root.path().join("Docs/probe.txt")).unwrap();
    if ready {
        return Some((root, dest));
    }
    assert!(
        std::env::var_os("PCTWIN_REQUIRE_UNIX_UNDO").is_none(),
        "undo cannot remove on this test drive, but these tests are required here"
    );
    eprintln!("skipped: the test drive is not one undo removes from");
    None
}

fn put(root: &tempfile::TempDir, stored: &str, bytes: &[u8]) {
    std::fs::write(root.path().join(stored), bytes).unwrap();
}

/// The copy's identity as PCTwin records it. PCTwin keeps each file it makes open until 10 ms
/// after it was made; the wait stands in for that.
fn id(dest: &Destination, stored: &str) -> FileId {
    std::thread::sleep(std::time::Duration::from_millis(11));
    dest.stat(stored).unwrap().unwrap().id
}

fn bytes_are(want: &'static [u8]) -> impl FnOnce(&mut std::fs::File) -> std::io::Result<bool> {
    move |f| {
        let mut got = Vec::new();
        f.read_to_end(&mut got)?;
        Ok(got == want)
    }
}

fn names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

fn hidden(dir: &Path) -> Vec<String> {
    names(dir)
        .into_iter()
        .filter(|n| n.starts_with(".pctwin-"))
        .collect()
}

/// A journal stand-in that keeps every step recorded, and can run something at a chosen step.
struct Record {
    steps: Vec<String>,
}

impl Record {
    fn new() -> Self {
        Self { steps: Vec::new() }
    }
}

fn step_text(s: &Step<'_>) -> String {
    format!("{s:?}")
}

/// Checks and removes the copy at `stored` in one go, with `at_step` run at each recorded step.
fn remove(
    dest: &Destination,
    stored: &str,
    file: FileId,
    verify: impl FnOnce(&mut std::fs::File) -> std::io::Result<bool>,
    record: &mut Record,
    at_step: &mut dyn FnMut(&Step<'_>) -> std::io::Result<()>,
) -> Removed {
    let copy = match dest.check_copy(stored, file, verify).unwrap() {
        Check::Ready(c) => c,
        Check::Done(r) => return r,
    };
    let private = pctwin_gate::private_name().unwrap();
    let j = journal();
    let permit = j.begin_undo().unwrap();
    let mut journal = |s: Step<'_>| {
        record.steps.push(step_text(&s));
        at_step(&s)
    };
    let mut cx = Context {
        kept_words: WORDS,
        room_for_words: 64,
        others_closed: false,
        journal: &mut journal,
    };
    dest.remove_checked(&permit, *copy, &private, &mut cx)
        .unwrap()
}

fn plain(dest: &Destination, stored: &str, file: FileId, want: &'static [u8]) -> Removed {
    remove(
        dest,
        stored,
        file,
        bytes_are(want),
        &mut Record::new(),
        &mut |_| Ok(()),
    )
}

#[test]
fn the_very_file_unchanged_is_removed_and_nothing_hidden_is_left() {
    let Some((root, dest)) = setup() else { return };
    put(&root, "Docs/a.txt", b"copy");
    let file = id(&dest, "Docs/a.txt");
    assert_eq!(plain(&dest, "Docs/a.txt", file, b"copy"), Removed::Removed);
    assert!(names(&root.path().join("Docs")).is_empty());
}

#[test]
fn a_changed_copy_is_kept_untouched() {
    let Some((root, dest)) = setup() else { return };
    put(&root, "Docs/a.txt", b"copy");
    let file = id(&dest, "Docs/a.txt");
    assert_eq!(plain(&dest, "Docs/a.txt", file, b"other"), Removed::Changed);
    assert_eq!(
        std::fs::read(root.path().join("Docs/a.txt")).unwrap(),
        b"copy"
    );
    assert!(hidden(&root.path().join("Docs")).is_empty());
}

#[test]
fn a_link_or_a_second_name_is_never_removed() {
    let Some((root, dest)) = setup() else { return };
    let docs = root.path().join("Docs");
    put(&root, "Docs/a.txt", b"copy");
    let file = id(&dest, "Docs/a.txt");
    std::fs::hard_link(docs.join("a.txt"), docs.join("theirs.txt")).unwrap();
    assert_eq!(plain(&dest, "Docs/a.txt", file, b"copy"), Removed::Linked);
    std::fs::remove_file(docs.join("theirs.txt")).unwrap();
    std::fs::rename(docs.join("a.txt"), docs.join("real.txt")).unwrap();
    std::os::unix::fs::symlink("real.txt", docs.join("a.txt")).unwrap();
    assert_eq!(
        plain(&dest, "Docs/a.txt", file, b"copy"),
        Removed::NotThatFile
    );
    assert_eq!(names(&docs), ["a.txt", "real.txt"]);
}

#[test]
fn a_file_an_app_keeps_open_is_kept() {
    let Some((root, dest)) = setup() else { return };
    for beside in [
        "a.db-wal",
        "a.db-shm",
        "a.db-journal",
        "a.db.lock",
        "~$a.db",
        ".~lock.a.db#",
        "a.db.lck",
    ] {
        put(&root, "Docs/a.db", b"copy");
        let file = id(&dest, "Docs/a.db");
        put(&root, &format!("Docs/{beside}"), b"");
        assert_eq!(
            plain(&dest, "Docs/a.db", file, b"copy"),
            Removed::AppKeepsOpen,
            "{beside}"
        );
        std::fs::remove_file(root.path().join("Docs").join(beside)).unwrap();
        std::fs::remove_file(root.path().join("Docs/a.db")).unwrap();
    }
    put(&root, "Docs/store.bin", b"SQLite format 3\0rest");
    let file = id(&dest, "Docs/store.bin");
    assert_eq!(
        plain(&dest, "Docs/store.bin", file, b"SQLite format 3\0rest"),
        Removed::AppKeepsOpen
    );
    put(&root, "Docs/mail.PST", b"copy");
    let file = id(&dest, "Docs/mail.PST");
    assert_eq!(
        plain(&dest, "Docs/mail.PST", file, b"copy"),
        Removed::AppKeepsOpen
    );
}

#[test]
fn a_copy_recorded_without_a_birth_time_is_never_removed() {
    let Some((root, dest)) = setup() else { return };
    put(&root, "Docs/a.txt", b"copy");
    let mut file = id(&dest, "Docs/a.txt");
    file.born = None;
    assert_eq!(
        plain(&dest, "Docs/a.txt", file, b"copy"),
        Removed::Unsupported
    );
    assert!(root.path().join("Docs/a.txt").exists());
}

/// Linux: another program with the file open (reading is enough) keeps it, untouched.
#[cfg(target_os = "linux")]
#[test]
fn a_file_another_program_has_open_is_kept_for_another_try() {
    let Some((root, dest)) = setup() else { return };
    put(&root, "Docs/a.txt", b"copy");
    let file = id(&dest, "Docs/a.txt");
    let _other = std::fs::File::open(root.path().join("Docs/a.txt")).unwrap();
    assert_eq!(plain(&dest, "Docs/a.txt", file, b"copy"), Removed::InUse);
    assert_eq!(names(&root.path().join("Docs")), ["a.txt"]);
}

/// A program's save that replaces the name just before the move: its new file is moved, found
/// not to be the copy, and put back under its name. Nothing of the person's is lost.
#[test]
fn a_new_version_saved_over_the_name_is_put_back_and_never_lost() {
    let Some((root, dest)) = setup() else { return };
    let docs = root.path().join("Docs");
    put(&root, "Docs/a.txt", b"copy");
    let file = id(&dest, "Docs/a.txt");
    let copy = match dest
        .check_copy("Docs/a.txt", file, bytes_are(b"copy"))
        .unwrap()
    {
        Check::Ready(c) => c,
        Check::Done(r) => panic!("{r:?}"),
    };
    // The private name is taken, so a new one is recorded; at that moment the person's app
    // saves a new version over the name.
    let private = pctwin_gate::private_name().unwrap();
    std::fs::write(docs.join(&private), b"planted").unwrap();
    let j = journal();
    let permit = j.begin_undo().unwrap();
    let mut steps = Vec::new();
    let mut journal = |s: Step<'_>| {
        if matches!(s, Step::NewPrivate { .. }) {
            std::fs::write(docs.join("new.tmp"), b"their new version").unwrap();
            std::fs::rename(docs.join("new.tmp"), docs.join("a.txt")).unwrap();
        }
        steps.push(step_text(&s));
        Ok(())
    };
    let mut cx = Context {
        kept_words: WORDS,
        room_for_words: 64,
        others_closed: false,
        journal: &mut journal,
    };
    let r = dest
        .remove_checked(&permit, *copy, &private, &mut cx)
        .unwrap();
    assert_eq!(r, Removed::NotThatFile);
    assert_eq!(
        std::fs::read(docs.join("a.txt")).unwrap(),
        b"their new version"
    );
    assert_eq!(std::fs::read(docs.join(&private)).unwrap(), b"planted");
    assert!(steps.iter().any(|s| s.contains("Putting")), "{steps:?}");
    assert_eq!(names(&docs), ["a.txt", private.as_str()]);
}

/// When the name is taken again as a file is being put back, it goes beside it, visibly.
#[test]
fn a_file_put_back_after_its_name_was_taken_goes_beside_it() {
    let Some((root, dest)) = setup() else { return };
    let docs = root.path().join("Docs");
    put(&root, "Docs/a.txt", b"copy");
    let file = id(&dest, "Docs/a.txt");
    let copy = match dest
        .check_copy("Docs/a.txt", file, bytes_are(b"copy"))
        .unwrap()
    {
        Check::Ready(c) => c,
        Check::Done(r) => panic!("{r:?}"),
    };
    let private = pctwin_gate::private_name().unwrap();
    std::fs::write(docs.join(&private), b"planted").unwrap();
    let j = journal();
    let permit = j.begin_undo().unwrap();
    let mut journal = |s: Step<'_>| {
        match s {
            Step::NewPrivate { .. } => {
                std::fs::write(docs.join("new.tmp"), b"their new version").unwrap();
                std::fs::rename(docs.join("new.tmp"), docs.join("a.txt")).unwrap();
            }
            Step::Putting { to: "a.txt", .. } => {
                std::fs::write(docs.join("a.txt"), b"and another").unwrap();
            }
            _ => {}
        }
        Ok(())
    };
    let mut cx = Context {
        kept_words: WORDS,
        room_for_words: 64,
        others_closed: false,
        journal: &mut journal,
    };
    let r = dest
        .remove_checked(&permit, *copy, &private, &mut cx)
        .unwrap();
    let beside = "a (kept by PCTwin undo).txt";
    assert_eq!(
        r,
        Removed::KeptBeside {
            at: format!("Docs/{beside}")
        }
    );
    assert_eq!(
        std::fs::read(docs.join(beside)).unwrap(),
        b"their new version"
    );
    assert_eq!(std::fs::read(docs.join("a.txt")).unwrap(), b"and another");
}

/// A step the journal cannot record is never taken.
#[test]
fn nothing_is_moved_without_its_record() {
    let Some((root, dest)) = setup() else { return };
    let docs = root.path().join("Docs");
    put(&root, "Docs/a.txt", b"copy");
    let file = id(&dest, "Docs/a.txt");
    let copy = match dest
        .check_copy("Docs/a.txt", file, bytes_are(b"copy"))
        .unwrap()
    {
        Check::Ready(c) => c,
        Check::Done(r) => panic!("{r:?}"),
    };
    let private = pctwin_gate::private_name().unwrap();
    std::fs::write(docs.join(&private), b"planted").unwrap();
    let j = journal();
    let permit = j.begin_undo().unwrap();
    let mut journal = |_: Step<'_>| Err(std::io::Error::other("disk full"));
    let mut cx = Context {
        kept_words: WORDS,
        room_for_words: 64,
        others_closed: false,
        journal: &mut journal,
    };
    assert!(
        dest.remove_checked(&permit, *copy, &private, &mut cx)
            .is_err()
    );
    assert_eq!(std::fs::read(docs.join("a.txt")).unwrap(), b"copy");
}

fn resolve_cx<'a>(journal: &'a mut dyn FnMut(Step<'_>) -> std::io::Result<()>) -> Context<'a> {
    Context {
        kept_words: WORDS,
        room_for_words: 64,
        others_closed: false,
        journal,
    }
}

fn dir_id(dest: &Destination, folder: &str) -> FileId {
    dest.folder_identity(folder).unwrap().unwrap()
}

/// After a crash at every point of a removal, the recorded private name is finished from the
/// disk, and finishing it twice ends the same way.
#[test]
fn a_part_way_removal_is_finished_from_the_disk() {
    let Some((root, dest)) = setup() else { return };
    let docs = root.path().join("Docs");
    let j = journal();
    let permit = j.begin_undo().unwrap();
    let mut ok = |_: Step<'_>| Ok(());
    let folder = dir_id(&dest, "Docs");
    let mut resolve = |private: &str, file: FileId, want: &'static [u8]| {
        dest.resolve_removing(
            &permit,
            "Docs/a.txt",
            file,
            folder,
            private,
            bytes_are(want),
            &mut resolve_cx(&mut ok),
        )
        .unwrap()
    };
    // Stopped before the move: still there.
    put(&root, "Docs/a.txt", b"copy");
    let file = id(&dest, "Docs/a.txt");
    let private = pctwin_gate::private_name().unwrap();
    assert_eq!(resolve(&private, file, b"copy"), Resolution::StillThere);
    // Stopped after the move: removed, and again it is already gone.
    std::fs::rename(docs.join("a.txt"), docs.join(&private)).unwrap();
    assert_eq!(resolve(&private, file, b"copy"), Resolution::Removed);
    assert_eq!(resolve(&private, file, b"copy"), Resolution::AlreadyGone);
    assert!(names(&docs).is_empty());
    // Changed while hidden: put back under its name.
    put(&root, "Docs/a.txt", b"copy");
    let file = id(&dest, "Docs/a.txt");
    std::fs::rename(docs.join("a.txt"), docs.join(&private)).unwrap();
    assert_eq!(resolve(&private, file, b"different"), Resolution::Home);
    assert_eq!(names(&docs), ["a.txt"]);
    // Changed while hidden and the name was taken: beside it, visibly.
    std::fs::rename(docs.join("a.txt"), docs.join(&private)).unwrap();
    put(&root, "Docs/a.txt", b"theirs");
    assert_eq!(
        resolve(&private, file, b"different"),
        Resolution::KeptAt {
            at: "Docs/a (kept by PCTwin undo).txt".into()
        }
    );
    assert!(hidden(&docs).is_empty());
}

/// Linux: a part-way removal of a file something still has open (a program that had it open
/// before PCTwin stopped) is not finished: it goes back under its name for another try.
#[cfg(target_os = "linux")]
#[test]
fn a_part_way_removal_of_a_file_still_open_elsewhere_goes_back_for_another_try() {
    let Some((root, dest)) = setup() else { return };
    let docs = root.path().join("Docs");
    let j = journal();
    let permit = j.begin_undo().unwrap();
    let folder = dir_id(&dest, "Docs");
    put(&root, "Docs/a.txt", b"copy");
    let file = id(&dest, "Docs/a.txt");
    let private = pctwin_gate::private_name().unwrap();
    std::fs::rename(docs.join("a.txt"), docs.join(&private)).unwrap();
    let other = std::fs::File::open(docs.join(&private)).unwrap();
    let mut ok = |_: Step<'_>| Ok(());
    let r = dest
        .resolve_removing(
            &permit,
            "Docs/a.txt",
            file,
            folder,
            &private,
            bytes_are(b"copy"),
            &mut resolve_cx(&mut ok),
        )
        .unwrap();
    assert_eq!(r, Resolution::InUse);
    assert_eq!(names(&docs), ["a.txt"]);
    drop(other);
    // Once it is closed, the next try removes it.
    let r = dest
        .resolve_removing(
            &permit,
            "Docs/a.txt",
            file,
            folder,
            &private,
            bytes_are(b"copy"),
            &mut resolve_cx(&mut ok),
        )
        .unwrap();
    assert_eq!(r, Resolution::StillThere);
}

/// A folder moved after a crash is found again by the exact private name; one that is gone is
/// never treated as finished.
#[test]
fn a_part_way_removal_in_a_moved_folder_is_found_and_a_missing_one_stays_open() {
    let Some((root, dest)) = setup() else { return };
    let j = journal();
    let permit = j.begin_undo().unwrap();
    std::fs::create_dir_all(root.path().join("Docs/Sub")).unwrap();
    put(&root, "Docs/Sub/a.txt", b"copy");
    let file = id(&dest, "Docs/Sub/a.txt");
    let folder = dir_id(&dest, "Docs/Sub");
    let private = pctwin_gate::private_name().unwrap();
    let sub = root.path().join("Docs/Sub");
    std::fs::rename(sub.join("a.txt"), sub.join(&private)).unwrap();
    std::fs::rename(&sub, root.path().join("Moved")).unwrap();
    let mut ok = |_: Step<'_>| Ok(());
    let r = dest
        .resolve_removing(
            &permit,
            "Docs/Sub/a.txt",
            file,
            folder,
            &private,
            bytes_are(b"copy"),
            &mut resolve_cx(&mut ok),
        )
        .unwrap();
    assert_eq!(r, Resolution::Removed);
    assert!(names(&root.path().join("Moved")).is_empty());
    // Recorded, but its folder is nowhere on this drive.
    let other = pctwin_gate::private_name().unwrap();
    let r = dest
        .resolve_removing(
            &permit,
            "Gone/a.txt",
            file,
            folder,
            &other,
            bytes_are(b"copy"),
            &mut resolve_cx(&mut ok),
        )
        .unwrap();
    assert_eq!(r, Resolution::FolderMissing);
}

#[test]
fn a_recorded_put_back_and_salvage_are_finished_visibly() {
    let Some((root, dest)) = setup() else { return };
    let docs = root.path().join("Docs");
    let j = journal();
    let permit = j.begin_undo().unwrap();
    let folder = dir_id(&dest, "Docs");
    let mut ok = |_: Step<'_>| Ok(());
    let private = pctwin_gate::private_name().unwrap();
    std::fs::write(docs.join(&private), b"theirs").unwrap();
    let r = dest
        .resolve_putting(
            &permit,
            "Docs/a.txt",
            folder,
            &private,
            "a.txt",
            &mut resolve_cx(&mut ok),
        )
        .unwrap();
    assert_eq!(r, Resolution::Home);
    // Again: already done.
    let r = dest
        .resolve_putting(
            &permit,
            "Docs/a.txt",
            folder,
            &private,
            "a.txt",
            &mut resolve_cx(&mut ok),
        )
        .unwrap();
    assert_eq!(r, Resolution::Home);
    // A salvage cut short: its bytes are named visibly, never over anything.
    let temp = format!(".pctwin-salvage-{}", "0".repeat(32));
    std::fs::write(docs.join(&temp), b"late bytes").unwrap();
    let r = dest
        .resolve_salvaging(
            &permit,
            "Docs/a.txt",
            folder,
            &temp,
            "a (kept by PCTwin undo).txt",
            &mut resolve_cx(&mut ok),
        )
        .unwrap();
    assert_eq!(
        r,
        Resolution::KeptAt {
            at: "Docs/a (kept by PCTwin undo).txt".into()
        }
    );
    assert_eq!(std::fs::read(docs.join("a.txt")).unwrap(), b"theirs");
    assert!(hidden(&docs).is_empty());
}

#[test]
fn private_names_are_random_and_recognised_exactly() {
    let a = pctwin_gate::private_name().unwrap();
    let b = pctwin_gate::private_name().unwrap();
    assert_ne!(a, b);
    assert!(a.starts_with(".pctwin-undo-") && a.len() == ".pctwin-undo-".len() + 32);
}

/// macOS: a file another program has open is kept, even one that only watches it for changes
/// (opened with O_EVTONLY): any program but PCTwin counts (finding 7).
#[cfg(target_os = "macos")]
#[test]
fn a_file_another_program_has_open_even_only_to_watch_it_is_kept() {
    use std::io::BufRead;
    let Some((root, dest)) = setup() else { return };
    for (how, flags) in [("read", "os.O_RDONLY"), ("watch", "0x8000")] {
        put(&root, "Docs/a.txt", b"copy");
        let file = id(&dest, "Docs/a.txt");
        let path = root.path().join("Docs/a.txt");
        let script = format!(
            "import os, sys, time\nfd = os.open(sys.argv[1], {flags})\nprint('open', flush=True)\ntime.sleep(60)\n"
        );
        let child = std::process::Command::new("python3")
            .arg("-c")
            .arg(&script)
            .arg(&path)
            .stdout(std::process::Stdio::piped())
            .spawn();
        let mut child = match child {
            Ok(c) => c,
            Err(e) => {
                assert!(
                    std::env::var_os("PCTWIN_REQUIRE_UNIX_UNDO").is_none(),
                    "python3 is needed for this test here: {e}"
                );
                eprintln!("skipped: no python3 to hold the file open ({e})");
                return;
            }
        };
        let mut line = String::new();
        std::io::BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        assert_eq!(line.trim(), "open", "{how}");
        let r = plain(&dest, "Docs/a.txt", file, b"copy");
        let _ = child.kill();
        let _ = child.wait();
        assert_eq!(r, Removed::InUse, "{how}");
        assert_eq!(std::fs::read(&path).unwrap(), b"copy", "{how}");
        std::fs::remove_file(&path).unwrap();
    }
}
