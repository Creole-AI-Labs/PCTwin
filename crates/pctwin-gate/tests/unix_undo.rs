//! Undo's removal on Linux and macOS: move aside first, then check (security design notes,
//! decided 9 October 2026). Since 13 October 2026 only Linux removes (a write lease proves
//! nobody else can write); since 20 October 2026 macOS moves a proven copy into the Trash on its
//! drive instead, and never deletes. The copy is checked through a held handle, its name moved,
//! never replacing, to a random private name the journal recorded first, proven there to be the
//! very file unchanged, and only then removed (Linux) or moved into the Trash (macOS). Anything
//! else goes back under its name or beside it; nothing is lost, nothing is left hidden, and
//! nothing but the proven copy reaches the Trash. A part-way removal is finished from the disk.
//!
//! On macOS these tests use a Trash of their own (`use_trash_home`), never the person's.
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
    #[cfg(target_os = "macos")]
    trash();
    let root = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
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

/// macOS: the Trash these tests use, `<a test home>/.Trash` on the same drive as their files, so
/// the person's real Trash is never touched. Shared by the tests in this run (names taken there
/// get Finder's numbers, as for the person).
#[cfg(target_os = "macos")]
fn trash() -> std::path::PathBuf {
    static HOME: std::sync::OnceLock<tempfile::TempDir> = std::sync::OnceLock::new();
    let home = HOME.get_or_init(|| tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap());
    pctwin_gate::use_trash_home(home.path().to_path_buf());
    home.path().join(".Trash")
}

/// macOS: whether any file in the test Trash holds exactly `bytes`.
#[cfg(target_os = "macos")]
fn trash_holds(bytes: &[u8]) -> bool {
    std::fs::read_dir(trash()).is_ok_and(|entries| {
        entries
            .flatten()
            .any(|e| std::fs::read(e.path()).is_ok_and(|b| b == bytes))
    })
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

#[cfg(target_os = "linux")]
#[test]
fn the_very_file_unchanged_is_removed_and_nothing_hidden_is_left() {
    let Some((root, dest)) = setup() else { return };
    put(&root, "Docs/a.txt", b"copy");
    let file = id(&dest, "Docs/a.txt");
    assert_eq!(plain(&dest, "Docs/a.txt", file, b"copy"), Removed::Removed);
    assert!(names(&root.path().join("Docs")).is_empty());
}

/// macOS: an unchanged copy goes to the Trash on its drive, under its own name there; nothing is
/// deleted and nothing hidden is left. The move into the Trash is journaled first, with the
/// private name it leaves and the Trash folder (decided 20 October 2026).
#[cfg(target_os = "macos")]
#[test]
fn on_macos_an_unchanged_copy_goes_to_the_trash_and_nothing_is_deleted() {
    let Some((root, dest)) = setup() else { return };
    put(&root, "Docs/to-trash-1.txt", b"copy for the trash 1");
    let file = id(&dest, "Docs/to-trash-1.txt");
    let mut record = Record::new();
    let r = remove(
        &dest,
        "Docs/to-trash-1.txt",
        file,
        bytes_are(b"copy for the trash 1"),
        &mut record,
        &mut |_| Ok(()),
    );
    let in_trash = trash().join("to-trash-1.txt");
    let shown = trash().to_str().unwrap().to_string();
    assert_eq!(
        r,
        Removed::Trashed {
            at: format!("{shown}/to-trash-1.txt")
        }
    );
    assert_eq!(std::fs::read(&in_trash).unwrap(), b"copy for the trash 1");
    assert!(names(&root.path().join("Docs")).is_empty());
    assert_eq!(record.steps.len(), 1, "{:?}", record.steps);
    assert!(
        record.steps[0].starts_with("Trashing")
            && record.steps[0].contains(&format!("{shown:?}"))
            && record.steps[0].contains("\"to-trash-1.txt\""),
        "{:?}",
        record.steps
    );
    // The person's Trash folder is theirs alone.
    use std::os::unix::fs::PermissionsExt;
    assert_eq!(
        std::fs::metadata(trash()).unwrap().permissions().mode() & 0o777,
        0o700
    );
}

/// macOS: a name already taken in the Trash is never replaced: the copy gets Finder's next
/// number, and what was there is untouched.
#[cfg(target_os = "macos")]
#[test]
fn on_macos_a_name_taken_in_the_trash_gets_the_next_number_and_nothing_is_replaced() {
    let Some((root, dest)) = setup() else { return };
    {
        use std::os::unix::fs::DirBuilderExt;
        let _ = std::fs::DirBuilder::new().mode(0o700).create(trash());
    }
    std::fs::write(trash().join("clash.txt"), b"already in the trash").unwrap();
    std::fs::write(trash().join("clash 2.txt"), b"also in the trash").unwrap();
    put(&root, "Docs/clash.txt", b"the clashing copy");
    let file = id(&dest, "Docs/clash.txt");
    let r = plain(&dest, "Docs/clash.txt", file, b"the clashing copy");
    let shown = trash().to_str().unwrap().to_string();
    assert_eq!(
        r,
        Removed::Trashed {
            at: format!("{shown}/clash 3.txt")
        }
    );
    assert_eq!(
        std::fs::read(trash().join("clash.txt")).unwrap(),
        b"already in the trash"
    );
    assert_eq!(
        std::fs::read(trash().join("clash 2.txt")).unwrap(),
        b"also in the trash"
    );
    assert_eq!(
        std::fs::read(trash().join("clash 3.txt")).unwrap(),
        b"the clashing copy"
    );
    assert!(names(&root.path().join("Docs")).is_empty());
}

/// macOS, the swap race: after the copy was checked, another program puts its own file in place
/// of the copy's name. That file is moved aside, found not to be the copy, and put back exactly
/// where it was; it never reaches the Trash.
#[cfg(target_os = "macos")]
#[test]
fn on_macos_a_file_swapped_in_after_the_check_never_reaches_the_trash() {
    let Some((root, dest)) = setup() else { return };
    let docs = root.path().join("Docs");
    put(&root, "Docs/swap.txt", b"the copy that was checked");
    let file = id(&dest, "Docs/swap.txt");
    let copy = match dest
        .check_copy(
            "Docs/swap.txt",
            file,
            bytes_are(b"the copy that was checked"),
        )
        .unwrap()
    {
        Check::Ready(c) => c,
        Check::Done(r) => panic!("{r:?}"),
    };
    // Between the check and the move: their file takes the name.
    std::fs::rename(docs.join("swap.txt"), docs.join("where the copy went.txt")).unwrap();
    std::fs::write(docs.join("swap.txt"), b"another program's file").unwrap();
    let private = pctwin_gate::private_name().unwrap();
    let j = journal();
    let permit = j.begin_undo().unwrap();
    let mut steps = Vec::new();
    let mut record = |s: Step<'_>| {
        steps.push(step_text(&s));
        Ok(())
    };
    let mut cx = Context {
        kept_words: WORDS,
        room_for_words: 64,
        journal: &mut record,
    };
    let r = dest
        .remove_checked(&permit, *copy, &private, &mut cx)
        .unwrap();
    assert!(matches!(r, Removed::NotThatFile | Removed::Gone), "{r:?}");
    assert_eq!(
        std::fs::read(docs.join("swap.txt")).unwrap(),
        b"another program's file"
    );
    assert_eq!(
        std::fs::read(docs.join("where the copy went.txt")).unwrap(),
        b"the copy that was checked"
    );
    assert!(!trash_holds(b"another program's file"));
    assert!(
        !steps.iter().any(|s| s.starts_with("Trashing")),
        "{steps:?}"
    );
    assert!(hidden(&docs).is_empty(), "{:?}", names(&docs));
}

/// macOS: in the moment between proving the copy under its private name and moving it into the
/// Trash, another program swaps its own file in under the private name. The private name is
/// proven again right before the move, so their file never reaches the Trash: it is put back
/// under the copy's name, visibly.
#[cfg(target_os = "macos")]
#[test]
fn on_macos_a_file_swapped_in_under_the_private_name_never_reaches_the_trash() {
    let Some((root, dest)) = setup() else { return };
    let docs = root.path().join("Docs");
    put(&root, "Docs/swap2.txt", b"the proven copy");
    let file = id(&dest, "Docs/swap2.txt");
    let mut record = Record::new();
    let docs2 = docs.clone();
    let r = remove(
        &dest,
        "Docs/swap2.txt",
        file,
        bytes_are(b"the proven copy"),
        &mut record,
        &mut |s| {
            if let Step::Trashing { private, .. } = s {
                std::fs::rename(docs2.join(private), docs2.join("moved by them.txt"))?;
                std::fs::write(docs2.join(private), b"their swapped-in file")?;
            }
            Ok(())
        },
    );
    assert_eq!(r, Removed::NotThatFile);
    assert_eq!(
        std::fs::read(docs.join("swap2.txt")).unwrap(),
        b"their swapped-in file"
    );
    assert_eq!(
        std::fs::read(docs.join("moved by them.txt")).unwrap(),
        b"the proven copy"
    );
    assert!(!trash_holds(b"their swapped-in file"));
    assert!(!trash_holds(b"the proven copy"));
    assert!(hidden(&docs).is_empty(), "{:?}", names(&docs));
}

/// macOS: a copy the person edits after the move is theirs: kept under its name, never sent to
/// the Trash, whether the edit came before undo, between the check and the move, or while the
/// copy was under its private name.
#[cfg(target_os = "macos")]
#[test]
fn on_macos_a_copy_edited_after_the_move_is_kept_and_never_trashed() {
    use std::io::Write;
    let Some((root, dest)) = setup() else { return };
    let docs = root.path().join("Docs");
    // Before undo: the check sees it.
    put(&root, "Docs/edit1.txt", b"as moved 1");
    let file = id(&dest, "Docs/edit1.txt");
    put(&root, "Docs/edit1.txt", b"edited by the person 1");
    assert_eq!(
        plain(&dest, "Docs/edit1.txt", file, b"as moved 1"),
        Removed::Changed
    );
    assert_eq!(
        std::fs::read(docs.join("edit1.txt")).unwrap(),
        b"edited by the person 1"
    );
    // Between the check and the move.
    put(&root, "Docs/edit2.txt", b"as moved 2");
    let file = id(&dest, "Docs/edit2.txt");
    let copy = match dest
        .check_copy("Docs/edit2.txt", file, bytes_are(b"as moved 2"))
        .unwrap()
    {
        Check::Ready(c) => c,
        Check::Done(r) => panic!("{r:?}"),
    };
    std::fs::OpenOptions::new()
        .append(true)
        .open(docs.join("edit2.txt"))
        .unwrap()
        .write_all(b" and edited 2")
        .unwrap();
    let private = pctwin_gate::private_name().unwrap();
    let j = journal();
    let permit = j.begin_undo().unwrap();
    let mut ok = |_: Step<'_>| Ok(());
    let r = dest
        .remove_checked(&permit, *copy, &private, &mut resolve_cx(&mut ok))
        .unwrap();
    assert_eq!(r, Removed::Changed);
    assert_eq!(
        std::fs::read(docs.join("edit2.txt")).unwrap(),
        b"as moved 2 and edited 2"
    );
    // While it was under its private name, right before the move into the Trash.
    put(&root, "Docs/edit3.txt", b"as moved 3");
    let file = id(&dest, "Docs/edit3.txt");
    let docs3 = docs.clone();
    let r = remove(
        &dest,
        "Docs/edit3.txt",
        file,
        bytes_are(b"as moved 3"),
        &mut Record::new(),
        &mut |s| {
            if let Step::Trashing { private, .. } = s {
                std::fs::OpenOptions::new()
                    .append(true)
                    .open(docs3.join(private))?
                    .write_all(b" and edited 3")?;
            }
            Ok(())
        },
    );
    assert_eq!(r, Removed::Changed);
    assert_eq!(
        std::fs::read(docs.join("edit3.txt")).unwrap(),
        b"as moved 3 and edited 3"
    );
    for edited in [
        &b"edited by the person 1"[..],
        b"as moved 2 and edited 2",
        b"as moved 3 and edited 3",
    ] {
        assert!(!trash_holds(edited));
    }
    assert!(hidden(&docs).is_empty(), "{:?}", names(&docs));
}

/// macOS: a move into the Trash cut short (the journal recorded the step, then PCTwin stopped)
/// is finished from the disk: before the move, the copy is proven and moved; after it, the copy
/// is found in the Trash; and finishing it twice ends the same way.
#[cfg(target_os = "macos")]
#[test]
fn on_macos_a_move_into_the_trash_cut_short_is_finished_from_the_disk() {
    let Some((root, dest)) = setup() else { return };
    let docs = root.path().join("Docs");
    put(&root, "Docs/cut.txt", b"cut short");
    let file = id(&dest, "Docs/cut.txt");
    let folder = dir_id(&dest, "Docs");
    // Stopped right after recording the move into the Trash: still under its private name.
    let mut seen = None;
    let copy = match dest
        .check_copy("Docs/cut.txt", file, bytes_are(b"cut short"))
        .unwrap()
    {
        Check::Ready(c) => c,
        Check::Done(r) => panic!("{r:?}"),
    };
    let private = pctwin_gate::private_name().unwrap();
    let j = journal();
    let permit = j.begin_undo().unwrap();
    let mut stop = |s: Step<'_>| match s {
        Step::Trashing { private, trash, to } => {
            seen = Some((private.to_string(), trash.to_string(), to.to_string()));
            Err(std::io::Error::other("PCTwin stopped here"))
        }
        _ => Ok(()),
    };
    assert!(
        dest.remove_checked(&permit, *copy, &private, &mut resolve_cx(&mut stop))
            .is_err()
    );
    let (hidden_as, trash_at, to) = seen.unwrap();
    assert_eq!(hidden(&docs), [hidden_as.as_str()]);
    let mut ok = |_: Step<'_>| Ok(());
    let mut resolve = || {
        dest.resolve_trashing(
            &permit,
            "Docs/cut.txt",
            file,
            folder,
            &hidden_as,
            &trash_at,
            &to,
            bytes_are(b"cut short"),
            &mut resolve_cx(&mut ok),
        )
        .unwrap()
    };
    let r = resolve();
    let Resolution::Trashed { at } = &r else {
        panic!("{r:?}")
    };
    assert_eq!(std::fs::read(at).unwrap(), b"cut short");
    assert!(names(&docs).is_empty());
    // Again: it is found in the Trash.
    assert_eq!(resolve(), r);
    // A recorded move whose copy is back under its own name (put back): looked at again.
    put(&root, "Docs/cut2.txt", b"cut short 2");
    let file2 = id(&dest, "Docs/cut2.txt");
    let other = pctwin_gate::private_name().unwrap();
    let r = dest
        .resolve_trashing(
            &permit,
            "Docs/cut2.txt",
            file2,
            folder,
            &other,
            &trash_at,
            "cut2.txt",
            bytes_are(b"cut short 2"),
            &mut resolve_cx(&mut ok),
        )
        .unwrap();
    assert_eq!(r, Resolution::StillThere);
    assert_eq!(
        std::fs::read(docs.join("cut2.txt")).unwrap(),
        b"cut short 2"
    );
}

/// macOS: a drive with no Trash (no `.Trashes` at its top: macOS has never made one there) keeps
/// the copy, untouched and unmoved. Once the drive has a `.Trashes`, the copy goes into the
/// person's own folder there, made for them alone. The drive is a disk image PCTwin's test makes
/// (APFS), so the test home's Trash is on another drive.
#[cfg(target_os = "macos")]
#[test]
fn on_macos_a_copy_on_another_drive_goes_to_that_drives_trash_or_is_kept() {
    let Some(_) = setup() else { return };
    let Some(image) = ApfsImage::new("PCTTRASH") else {
        return;
    };
    let mnt = image.mount.clone();
    std::fs::create_dir(mnt.join("Docs")).unwrap();
    let dest = Destination::open(&mnt).unwrap();
    let _ = std::fs::remove_dir_all(mnt.join(".Trashes"));
    std::fs::write(mnt.join("Docs/d.txt"), b"on another drive").unwrap();
    let file = id(&dest, "Docs/d.txt");
    let mut record = Record::new();
    let r = remove(
        &dest,
        "Docs/d.txt",
        file,
        bytes_are(b"on another drive"),
        &mut record,
        &mut |_| Ok(()),
    );
    assert_eq!(r, Removed::NoTrash);
    assert!(record.steps.is_empty(), "{:?}", record.steps);
    assert_eq!(names(&mnt.join("Docs")), ["d.txt"]);
    // macOS's own `.Trashes`: write-only for everyone, with the sticky mark.
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir(mnt.join(".Trashes")).unwrap();
    std::fs::set_permissions(
        mnt.join(".Trashes"),
        std::fs::Permissions::from_mode(0o1333),
    )
    .unwrap();
    let uid = std::process::Command::new("id").arg("-u").output().unwrap();
    let uid = String::from_utf8(uid.stdout).unwrap().trim().to_string();
    // A folder there for this person that others may write to is not theirs alone: kept.
    let theirs = mnt.join(".Trashes").join(&uid);
    std::fs::create_dir(&theirs).unwrap();
    std::fs::set_permissions(&theirs, std::fs::Permissions::from_mode(0o777)).unwrap();
    assert_eq!(
        plain(&dest, "Docs/d.txt", file, b"on another drive"),
        Removed::NoTrash
    );
    assert_eq!(names(&mnt.join("Docs")), ["d.txt"]);
    assert!(names(&theirs).is_empty());
    std::fs::set_permissions(&theirs, std::fs::Permissions::from_mode(0o700)).unwrap();
    let r = plain(&dest, "Docs/d.txt", file, b"on another drive");
    let mine = std::fs::canonicalize(&mnt)
        .unwrap()
        .join(".Trashes")
        .join(&uid);
    assert_eq!(
        r,
        Removed::Trashed {
            at: format!("{}/d.txt", mine.to_str().unwrap())
        }
    );
    assert_eq!(
        std::fs::read(mine.join("d.txt")).unwrap(),
        b"on another drive"
    );
    assert_eq!(
        std::fs::metadata(&mine).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert!(names(&mnt.join("Docs")).is_empty());
}

/// macOS, on GitHub's runners only: the person's real Trash (`~/.Trash`), found the way the app
/// finds it, takes a proven copy. Run in a child process (the test hook cannot be unset), which
/// then takes its own file back out of the Trash.
#[cfg(target_os = "macos")]
#[test]
fn on_macos_the_persons_own_trash_takes_a_proven_copy_on_ci() {
    if std::env::var_os("GITHUB_ACTIONS").is_none() {
        eprintln!("skipped: only on CI runners, never in a person's real Trash");
        return;
    }
    let out = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "real_trash_child",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("PCTWIN_REAL_TRASH_CHILD", "1")
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    eprintln!("{text}");
    assert!(out.status.success(), "{text}");
    assert!(text.contains("1 passed"), "{text}");
}

/// The child of the test above (it alone uses the real home folder).
#[cfg(target_os = "macos")]
#[test]
#[ignore = "run only by on_macos_the_persons_own_trash_takes_a_proven_copy_on_ci"]
fn real_trash_child() {
    if std::env::var_os("PCTWIN_REAL_TRASH_CHILD").is_none() {
        return;
    }
    let root = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    std::fs::create_dir(root.path().join("Docs")).unwrap();
    let dest = Destination::open(root.path()).unwrap();
    let name = format!("pctwin-ci-{}.txt", std::process::id());
    let stored = format!("Docs/{name}");
    put(&root, &stored, b"for the real trash");
    let file = id(&dest, &stored);
    let r = plain(&dest, &stored, file, b"for the real trash");
    eprintln!("real Trash: {r:?}");
    let Removed::Trashed { at } = r else {
        panic!("{r:?}")
    };
    let home = std::env::home_dir().unwrap();
    assert!(
        at.starts_with(&format!("{}/.Trash/", home.to_str().unwrap())),
        "{at}"
    );
    assert_eq!(std::fs::read(&at).unwrap(), b"for the real trash");
    std::fs::remove_file(&at).unwrap();
}

/// A disk image made and attached for a test, detached and removed when dropped.
#[cfg(target_os = "macos")]
struct ApfsImage {
    _dir: tempfile::TempDir,
    mount: std::path::PathBuf,
}

#[cfg(target_os = "macos")]
impl ApfsImage {
    fn new(volume: &str) -> Option<Self> {
        let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
        let image = dir.path().join("drive.dmg");
        let mount = dir.path().join("mnt");
        std::fs::create_dir(&mount).unwrap();
        let ok = |args: &[&std::ffi::OsStr]| {
            std::process::Command::new("hdiutil")
                .args(args)
                .output()
                .is_ok_and(|o| o.status.success())
        };
        let made = ok(&[
            "create".as_ref(),
            "-size".as_ref(),
            "20m".as_ref(),
            "-fs".as_ref(),
            "APFS".as_ref(),
            "-volname".as_ref(),
            volume.as_ref(),
            image.as_os_str(),
        ]) && ok(&[
            "attach".as_ref(),
            image.as_os_str(),
            "-mountpoint".as_ref(),
            mount.as_os_str(),
            "-nobrowse".as_ref(),
        ]);
        if !made {
            assert!(
                std::env::var_os("PCTWIN_REQUIRE_UNIX_UNDO").is_none(),
                "hdiutil could not make a test drive here"
            );
            eprintln!("skipped: hdiutil could not make a test drive");
            return None;
        }
        Some(Self { _dir: dir, mount })
    }
}

#[cfg(target_os = "macos")]
impl Drop for ApfsImage {
    fn drop(&mut self) {
        let _ = std::process::Command::new("hdiutil")
            .arg("detach")
            .arg(&self.mount)
            .arg("-force")
            .output();
    }
}

/// FAT and exFAT give file numbers again to new files, so PCTwin cannot tell its own copy there:
/// it is kept without being read or moved, and the undo screen is told so first (decided 20
/// October 2026). Shown here with the drive kind forced; real drives in `real_fat.rs`.
#[test]
fn a_copy_on_fat_or_exfat_is_kept_and_the_undo_screen_is_told_first() {
    use pctwin_gate::{DriveKind, DriveProfile, UndoHere};
    let Some((root, dest)) = setup() else { return };
    assert_ne!(dest.undo_here(), UndoHere::KeepsFileNumbersReused);
    put(&root, "Docs/fat.txt", b"on a fat drive");
    let file = id(&dest, "Docs/fat.txt");
    for kind in [DriveKind::Fat, DriveKind::ExFat] {
        let fat = dest
            .try_clone()
            .unwrap()
            .with_profile(DriveProfile::of(kind));
        assert_eq!(fat.undo_here(), UndoHere::KeepsFileNumbersReused);
        let mut read = false;
        let r = remove(
            &fat,
            "Docs/fat.txt",
            file,
            |_| {
                read = true;
                Ok(true)
            },
            &mut Record::new(),
            &mut |_| Ok(()),
        );
        assert_eq!(r, Removed::NumbersReused, "{kind:?}");
        assert!(!read, "nothing was read");
    }
    assert_eq!(names(&root.path().join("Docs")), ["fat.txt"]);
    #[cfg(target_os = "macos")]
    assert!(!trash_holds(b"on a fat drive"));
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
    // Nothing of PCTwin's is left hidden: the one private-looking name is the file this test
    // planted there, untouched (`names` is sorted, so it comes first).
    assert_eq!(hidden(&docs), [private.as_str()]);
    assert_eq!(names(&docs), [private.as_str(), "a.txt"]);
    #[cfg(target_os = "macos")]
    assert!(!trash_holds(b"their new version"));
}

/// A second name made for the copy just as it is moved aside keeps it, and says why: the count
/// of names is looked at before the change time (making a name moves the change time too).
#[test]
fn a_second_name_made_as_the_copy_is_moved_aside_keeps_it_as_linked() {
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
    // The private name is taken, so a new one is recorded; at that moment another name is made.
    let private = pctwin_gate::private_name().unwrap();
    std::fs::write(docs.join(&private), b"planted").unwrap();
    let j = journal();
    let permit = j.begin_undo().unwrap();
    let mut journal = |s: Step<'_>| {
        if matches!(s, Step::NewPrivate { .. }) {
            std::fs::hard_link(docs.join("a.txt"), docs.join("theirs.txt")).unwrap();
        }
        Ok(())
    };
    let mut cx = Context {
        kept_words: WORDS,
        room_for_words: 64,
        journal: &mut journal,
    };
    let r = dest
        .remove_checked(&permit, *copy, &private, &mut cx)
        .unwrap();
    assert_eq!(r, Removed::Linked);
    assert_eq!(std::fs::read(docs.join("a.txt")).unwrap(), b"copy");
    assert_eq!(std::fs::read(docs.join("theirs.txt")).unwrap(), b"copy");
    assert_eq!(hidden(&docs), [private.as_str()]);
}

/// A copy removed by someone else after it was checked is gone. That is decided from the held
/// file (no names left), never from a check by its path that can no longer find it (macOS asks
/// which programs have a file open by its path).
#[test]
fn a_copy_removed_by_someone_else_after_it_was_checked_is_gone() {
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
    std::fs::remove_file(docs.join("a.txt")).unwrap();
    let private = pctwin_gate::private_name().unwrap();
    let j = journal();
    let permit = j.begin_undo().unwrap();
    let mut nothing = |_: Step<'_>| Ok(());
    let mut cx = Context {
        kept_words: WORDS,
        room_for_words: 64,
        journal: &mut nothing,
    };
    let r = dest
        .remove_checked(&permit, *copy, &private, &mut cx)
        .unwrap();
    assert_eq!(r, Removed::Gone);
    assert!(names(&docs).is_empty());
}

/// Linux: while another holds a write lease on the copy (another undo try at the same moment, or
/// a file server), an open that never waits is refused. That is "in use", for another try, never
/// an error.
#[cfg(target_os = "linux")]
#[test]
#[cfg_attr(
    target_os = "macos",
    ignore = "macOS never removes a copy (13 October 2026): no move aside to test"
)]
fn a_copy_another_holds_a_lease_on_is_in_use_never_an_error() {
    let Some((root, dest)) = setup() else { return };
    put(&root, "Docs/a.txt", b"copy");
    let file = id(&dest, "Docs/a.txt");
    let other = std::fs::File::open(root.path().join("Docs/a.txt")).unwrap();
    assert!(matches!(
        pctwin_lease::write_lease(&other),
        pctwin_lease::Lease::Held
    ));
    assert_eq!(plain(&dest, "Docs/a.txt", file, b"copy"), Removed::InUse);
    assert_eq!(names(&root.path().join("Docs")), ["a.txt"]);
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
        journal,
    }
}

fn dir_id(dest: &Destination, folder: &str) -> FileId {
    dest.folder_identity(folder).unwrap().unwrap()
}

/// After a crash at every point of a removal, the recorded private name is finished from the
/// disk, and finishing it twice ends the same way. On macOS a proven copy is finished by moving
/// it into the Trash.
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
    // Stopped after the move: removed (macOS: moved into the Trash), and again it is already gone.
    std::fs::rename(docs.join("a.txt"), docs.join(&private)).unwrap();
    let r = resolve(&private, file, b"copy");
    #[cfg(target_os = "macos")]
    assert!(
        matches!(&r, Resolution::Trashed { at } if std::fs::read(at).unwrap() == b"copy"),
        "{r:?}"
    );
    #[cfg(not(target_os = "macos"))]
    assert_eq!(r, Resolution::Removed);
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
    #[cfg(target_os = "macos")]
    assert!(matches!(r, Resolution::Trashed { .. }), "{r:?}");
    #[cfg(not(target_os = "macos"))]
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
            true,
            &mut resolve_cx(&mut ok),
        )
        .unwrap();
    assert_eq!(
        r,
        Resolution::SavedAgain {
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

// ---- security review, 10 October 2026 -------------------------------------------------------

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

/// A put-back that ended under the copy's own name, with the copy itself there, is not "changed":
/// the normal checks run again (finding 2). Only a file that is not the copy is `Home`.
#[test]
fn a_put_back_home_of_the_copy_itself_is_looked_at_again_not_called_changed() {
    let Some((root, dest)) = setup() else { return };
    let docs = root.path().join("Docs");
    let j = journal();
    let permit = j.begin_undo().unwrap();
    let folder = dir_id(&dest, "Docs");
    let mut ok = |_: Step<'_>| Ok(());
    put(&root, "Docs/a.txt", b"copy");
    let file = id(&dest, "Docs/a.txt");
    let private = pctwin_gate::private_name().unwrap();
    // Stopped after the rename back (private name empty), and stopped before it.
    for moved_back in [true, false] {
        if !moved_back {
            std::fs::rename(docs.join("a.txt"), docs.join(&private)).unwrap();
        }
        let r = dest
            .resolve_putting(
                &permit,
                "Docs/a.txt",
                file,
                folder,
                &private,
                "a.txt",
                &mut resolve_cx(&mut ok),
            )
            .unwrap();
        assert_eq!(r, Resolution::StillThere, "moved back: {moved_back}");
        assert_eq!(names(&docs), ["a.txt"]);
    }
    // Something that is not the copy, put back under the name: Home.
    std::fs::remove_file(docs.join("a.txt")).unwrap();
    put(&root, "Docs/a.txt", b"theirs");
    let r = dest
        .resolve_putting(
            &permit,
            "Docs/a.txt",
            file,
            folder,
            &private,
            "a.txt",
            &mut resolve_cx(&mut ok),
        )
        .unwrap();
    assert_eq!(r, Resolution::Home);
}

/// Something that is not a regular file under a private name (a link, a folder) is never taken
/// for "nothing there": it goes back under the copy's name, visibly (finding 6).
#[test]
fn something_that_is_not_a_file_under_a_private_name_is_put_back_never_ignored() {
    let Some((root, dest)) = setup() else { return };
    let docs = root.path().join("Docs");
    let j = journal();
    let permit = j.begin_undo().unwrap();
    let folder = dir_id(&dest, "Docs");
    let mut ok = |_: Step<'_>| Ok(());
    put(&root, "Docs/probe2.txt", b"x");
    let file = id(&dest, "Docs/probe2.txt");
    std::fs::remove_file(docs.join("probe2.txt")).unwrap();
    for kind in ["link", "folder"] {
        let private = pctwin_gate::private_name().unwrap();
        if kind == "link" {
            std::os::unix::fs::symlink("elsewhere", docs.join(&private)).unwrap();
        } else {
            std::fs::create_dir(docs.join(&private)).unwrap();
        }
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
        assert_eq!(r, Resolution::Home, "{kind}");
        assert!(hidden(&docs).is_empty(), "{kind}: {:?}", names(&docs));
        let meta = std::fs::symlink_metadata(docs.join("a.txt")).unwrap();
        assert_eq!(meta.is_symlink(), kind == "link", "{kind}");
        assert_eq!(meta.is_dir(), kind == "folder", "{kind}");
        if kind == "link" {
            std::fs::remove_file(docs.join("a.txt")).unwrap();
        } else {
            std::fs::remove_dir(docs.join("a.txt")).unwrap();
        }
    }
}

/// A name the drive cannot even look at is an error, never "already gone" (finding 5).
#[test]
fn a_name_that_cannot_be_looked_at_is_an_error_never_already_gone() {
    let Some((_root, dest)) = setup() else { return };
    let j = journal();
    let permit = j.begin_undo().unwrap();
    let folder = dir_id(&dest, "Docs");
    let mut ok = |_: Step<'_>| Ok(());
    let private = pctwin_gate::private_name().unwrap();
    let too_long = "x".repeat(300);
    let r = dest.resolve_putting(
        &permit,
        "Docs/a.txt",
        folder,
        folder,
        &private,
        &too_long,
        &mut resolve_cx(&mut ok),
    );
    assert!(r.is_err(), "{r:?}");
    let temp = format!(".pctwin-salvage-{}", "2".repeat(32));
    let r = dest.resolve_salvaging(
        &permit,
        "Docs/a.txt",
        folder,
        &temp,
        &too_long,
        false,
        &mut resolve_cx(&mut ok),
    );
    assert!(r.is_err(), "{r:?}");
}

/// Linux: every "already done" answer of the resolver flushes the folder first, so a folder that
/// cannot be flushed (here: one PCTwin may not read) is an error, not a quiet "done" (finding 5).
#[cfg(target_os = "linux")]
#[test]
fn an_already_done_resolution_flushes_its_folder_first() {
    use std::os::unix::fs::PermissionsExt;
    let Some((root, dest)) = setup() else { return };
    let docs = root.path().join("Docs");
    let j = journal();
    let permit = j.begin_undo().unwrap();
    let folder = dir_id(&dest, "Docs");
    put(&root, "Docs/a.txt", b"copy");
    let file = id(&dest, "Docs/a.txt");
    std::fs::remove_file(docs.join("a.txt")).unwrap();
    let mut ok = |_: Step<'_>| Ok(());
    let private = pctwin_gate::private_name().unwrap();
    std::fs::set_permissions(&docs, std::fs::Permissions::from_mode(0o300)).unwrap();
    // Root reads any folder: then there is nothing to prove here.
    let unreadable = std::fs::read_dir(&docs).is_err();
    let removing = dest.resolve_removing(
        &permit,
        "Docs/a.txt",
        file,
        folder,
        &private,
        bytes_are(b"copy"),
        &mut resolve_cx(&mut ok),
    );
    let putting = dest.resolve_putting(
        &permit,
        "Docs/a.txt",
        file,
        folder,
        &private,
        "a.txt",
        &mut resolve_cx(&mut ok),
    );
    let salvaging = dest.resolve_salvaging(
        &permit,
        "Docs/a.txt",
        folder,
        &format!(".pctwin-salvage-{}", "3".repeat(32)),
        "a (kept by PCTwin undo).txt",
        true,
        &mut resolve_cx(&mut ok),
    );
    std::fs::set_permissions(&docs, std::fs::Permissions::from_mode(0o755)).unwrap();
    if unreadable {
        assert!(removing.is_err(), "{removing:?}");
        assert!(putting.is_err(), "{putting:?}");
        assert!(salvaging.is_err(), "{salvaging:?}");
    } else {
        eprintln!("skipped: running as a user who can read any folder");
    }
}

/// A salvage that PCTwin stopped while copying is published under a visible name as "may be
/// incomplete", readable by its owner; one recorded complete is "saved again" (finding 1).
#[test]
fn a_salvage_cut_short_is_published_as_maybe_incomplete() {
    use std::os::unix::fs::PermissionsExt;
    let Some((root, dest)) = setup() else { return };
    let docs = root.path().join("Docs");
    let j = journal();
    let permit = j.begin_undo().unwrap();
    let folder = dir_id(&dest, "Docs");
    let mut ok = |_: Step<'_>| Ok(());
    for (n, complete, mode) in [(4u8, false, 0o000), (5, false, 0o644), (6, true, 0o640)] {
        let temp = format!(".pctwin-salvage-{}", n.to_string().repeat(32));
        std::fs::write(docs.join(&temp), b"late bytes").unwrap();
        std::fs::set_permissions(docs.join(&temp), std::fs::Permissions::from_mode(mode)).unwrap();
        let to = format!("a{n} (kept by PCTwin undo).txt");
        let stored = format!("Docs/a{n}.txt");
        let mut resolve = || {
            dest.resolve_salvaging(
                &permit,
                &stored,
                folder,
                &temp,
                &to,
                complete,
                &mut resolve_cx(&mut ok),
            )
            .unwrap()
        };
        let r = resolve();
        let at = format!("Docs/{to}");
        if complete {
            assert_eq!(r, Resolution::SavedAgain { at });
        } else {
            assert_eq!(r, Resolution::SavedMaybeIncomplete { at });
        }
        // Again, once published: the same answer.
        assert_eq!(resolve(), r);
        let meta = std::fs::metadata(docs.join(&to)).unwrap();
        // Readable by its owner, whatever it had while being copied.
        assert_ne!(meta.permissions().mode() & 0o400, 0, "{n}");
        assert_eq!(std::fs::read(docs.join(&to)).unwrap(), b"late bytes");
    }
    assert!(hidden(&docs).is_empty());
}

/// macOS: a removal cut short after the move aside is never finished by removing the file: a
/// proven copy goes on to the Trash, and one that changed while hidden goes back under its name.
#[cfg(target_os = "macos")]
#[test]
fn on_macos_a_part_way_removal_goes_to_the_trash_or_back_never_deleted() {
    let Some((root, dest)) = setup() else { return };
    let docs = root.path().join("Docs");
    let j = journal();
    let permit = j.begin_undo().unwrap();
    let mut ok = |_: Step<'_>| Ok(());
    let folder = dir_id(&dest, "Docs");
    put(&root, "Docs/a.txt", b"copy");
    let file = id(&dest, "Docs/a.txt");
    let private = pctwin_gate::private_name().unwrap();
    std::fs::rename(docs.join("a.txt"), docs.join(&private)).unwrap();
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
    assert!(
        matches!(&r, Resolution::Trashed { at } if std::fs::read(at).unwrap() == b"copy"),
        "{r:?}"
    );
    assert!(names(&docs).is_empty());
    // Changed while hidden: back under its name, kept, and not in the Trash.
    put(&root, "Docs/b.txt", b"copy b");
    let file = id(&dest, "Docs/b.txt");
    let private = pctwin_gate::private_name().unwrap();
    std::fs::rename(docs.join("b.txt"), docs.join(&private)).unwrap();
    let r = dest
        .resolve_removing(
            &permit,
            "Docs/b.txt",
            file,
            folder,
            &private,
            bytes_are(b"something else"),
            &mut resolve_cx(&mut ok),
        )
        .unwrap();
    assert_eq!(r, Resolution::Home);
    assert_eq!(std::fs::read(docs.join("b.txt")).unwrap(), b"copy b");
    assert!(!trash_holds(b"copy b"));
}

/// A file put at the copy's name in the moment undo moves it aside (the copy itself moved to
/// another name, unchanged, with one name) is found not to be the copy right after the move:
/// it goes straight back, and on macOS nothing of it is ever recorded as going to the Trash.
#[test]
fn a_file_put_at_the_name_while_it_is_moved_aside_goes_straight_back() {
    let Some((root, dest)) = setup() else { return };
    let docs = root.path().join("Docs");
    put(&root, "Docs/aside.txt", b"copy");
    let file = id(&dest, "Docs/aside.txt");
    let copy = match dest
        .check_copy("Docs/aside.txt", file, bytes_are(b"copy"))
        .unwrap()
    {
        Check::Ready(c) => c,
        Check::Done(r) => panic!("{r:?}"),
    };
    // The first private name is taken, so a new one is recorded; at that moment the copy is
    // renamed away and another file of the same size takes its name.
    let private = pctwin_gate::private_name().unwrap();
    std::fs::write(docs.join(&private), b"planted").unwrap();
    let j = journal();
    let permit = j.begin_undo().unwrap();
    let mut steps = Vec::new();
    let mut journal = |s: Step<'_>| {
        if matches!(s, Step::NewPrivate { .. }) {
            std::fs::rename(docs.join("aside.txt"), docs.join("moved away.txt")).unwrap();
            std::fs::write(docs.join("aside.txt"), b"COPY").unwrap();
        }
        steps.push(step_text(&s));
        Ok(())
    };
    let mut cx = Context {
        kept_words: WORDS,
        room_for_words: 64,
        journal: &mut journal,
    };
    let r = dest
        .remove_checked(&permit, *copy, &private, &mut cx)
        .unwrap();
    assert_eq!(r, Removed::NotThatFile);
    assert_eq!(std::fs::read(docs.join("aside.txt")).unwrap(), b"COPY");
    assert_eq!(std::fs::read(docs.join("moved away.txt")).unwrap(), b"copy");
    assert!(
        !steps.iter().any(|s| s.starts_with("Trashing")),
        "{steps:?}"
    );
    assert_eq!(hidden(&docs), [private.as_str()]);
}
