//! Filling a file in under its real name, for drives with no safe rename (security design
//! notes, "Drives with no safe rename", decided 15 October 2026): forced here on the test
//! machine's own drive through a drive profile, so every system runs it. The real drives that
//! need it (exFAT on a Mac) run it in the real-drive tests.
// Tests make and read files to test on; the no-file-changes fence is for the shipped code.
#![allow(clippy::disallowed_methods)]

use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::Arc;

mod common;

use pctwin_gate::{
    Claim, Destination, DriveKind, DriveProfile, FileId, FillPlan, FillSteps, GateError,
    IncomingPath, Naming, Proof, Sealed,
};
#[cfg(any(windows, target_os = "macos"))]
use pctwin_gate::{ShowCheck, Shown};

/// A drive that fills files in: exFAT where the system can tag and hide a file (Windows, macOS);
/// on Linux, which can do neither, a drive that keeps identities (an exFAT drive would be refused
/// there, nothing proving its files PCTwin's after a restart).
fn fill_in_kind() -> DriveKind {
    if cfg!(any(windows, target_os = "macos")) {
        DriveKind::ExFat
    } else {
        DriveKind::Ext4
    }
}

fn fill_in(root: &Path) -> Destination {
    Destination::open(root)
        .unwrap()
        .with_profile(DriveProfile::of(fill_in_kind()).with_naming(Naming::FillIn))
}

fn sealed<'d>(dest: &'d Destination, path: &str, bytes: &[u8]) -> Sealed<'d> {
    let mut file = dest
        .create_file(&IncomingPath::parse(path).unwrap(), bytes.len() as u64)
        .unwrap();
    file.write_all(bytes).unwrap();
    common::seal(file).unwrap()
}

fn names(dir: &Path) -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    out.sort();
    out
}

/// What the journal was told, in order.
#[derive(Default)]
struct Steps {
    log: Vec<String>,
    /// The bytes expected on read-back.
    expect: Vec<u8>,
    /// Runs right after the read-back (to act as another program there).
    after_read: Option<Box<dyn FnMut()>>,
    /// Runs as the fill is recorded filled (to look at the file then).
    at_filled: Option<Box<dyn FnMut()>>,
}

impl FillSteps for Steps {
    fn made(&mut self, _: FileId) -> io::Result<()> {
        self.log.push("made".into());
        Ok(())
    }
    fn read_back(&mut self, file: &mut dyn Read) -> io::Result<bool> {
        let mut got = Vec::new();
        file.read_to_end(&mut got)?;
        self.log.push("read back".into());
        if let Some(act) = self.after_read.as_mut() {
            act();
        }
        Ok(got == self.expect)
    }
    fn filled(&mut self) -> io::Result<()> {
        self.log.push("filled".into());
        if let Some(look) = self.at_filled.as_mut() {
            look();
        }
        Ok(())
    }
    fn cleared(&mut self) -> io::Result<()> {
        self.log.push("cleared".into());
        Ok(())
    }
}

fn steps(expect: &[u8]) -> Steps {
    Steps {
        expect: expect.to_vec(),
        ..Steps::default()
    }
}

#[test]
fn a_file_filled_in_lands_whole_and_the_steps_are_recorded_in_order() {
    let root = tempfile::tempdir().unwrap();
    let dest = fill_in(root.path());
    let bytes = vec![7u8; 3000];
    let s = sealed(&dest, "Docs/a.bin", &bytes);
    assert_eq!(s.naming().unwrap(), Naming::FillIn);
    let plan = s.fill_plan().unwrap();
    // exFAT's identities change: the creation-time tag proves the file (Windows, macOS); the
    // Linux stand-in keeps identities, which prove it there.
    let expected = if cfg!(any(windows, target_os = "macos")) {
        Proof::Token
    } else {
        Proof::StableId
    };
    assert_eq!(plan.proof, expected);
    let name = s.next_name().unwrap();
    assert_eq!(name, "Docs/a.bin");
    let mut st = steps(&bytes);
    let Claim::Named(claimed) = s.fill_as(&name, &plan, &mut st) else {
        panic!("not named");
    };
    assert_eq!(st.log, ["made", "read back", "filled"]);
    // The temporary file stays until the journal has committed, then goes.
    assert_eq!(names(&root.path().join("Docs")).len(), 2);
    let done = claimed.keep();
    assert_eq!(done.final_path, "Docs/a.bin");
    assert_eq!(
        std::fs::read(root.path().join("Docs/a.bin")).unwrap(),
        bytes
    );
    assert_eq!(names(&root.path().join("Docs")), ["a.bin"]);
}

/// On a drive that keeps identities, the identity recorded once the file is made proves it.
#[test]
fn a_drive_that_keeps_identities_proves_the_file_by_it() {
    let root = tempfile::tempdir().unwrap();
    let dest = Destination::open(root.path())
        .unwrap()
        .with_profile(DriveProfile::of(DriveKind::Ntfs).with_naming(Naming::FillIn));
    let s = sealed(&dest, "k.txt", b"k");
    assert_eq!(s.fill_plan().unwrap().proof, Proof::StableId);
}

/// Without a journal (a plain finish), a drive that fills in still lands files whole.
#[test]
fn a_plain_finish_fills_in_on_such_a_drive() {
    let root = tempfile::tempdir().unwrap();
    let dest = fill_in(root.path());
    let mut file = dest
        .create_file(&IncomingPath::parse("b.txt").unwrap(), 3)
        .unwrap();
    file.write_all(b"abc").unwrap();
    assert_eq!(common::finish(file).unwrap().final_path, "b.txt");
    assert_eq!(std::fs::read(root.path().join("b.txt")).unwrap(), b"abc");
    assert_eq!(names(root.path()), ["b.txt"]);
}

/// While it is filled, the file is hidden and carries PCTwin's tag (where the system can do
/// both); once complete and checked it is shown, and gets back a real creation date (the checked
/// copy's, as a file named by a move keeps), never the tag.
#[cfg(any(windows, target_os = "macos"))]
#[test]
fn the_file_is_hidden_and_tagged_while_filled_then_shown_with_its_real_date() {
    let root = tempfile::tempdir().unwrap();
    let dest = Arc::new(fill_in(root.path()));
    let s = sealed(&dest, "c.txt", b"xyz");
    let real = std::fs::metadata(root.path().join(s.temp_path()))
        .unwrap()
        .created()
        .unwrap();
    let plan = s.fill_plan().unwrap();
    assert!(plan.tag.hidden);
    let tag = plan.tag.born.expect("this system sets creation times");
    let seen = Arc::new(std::sync::Mutex::new(None));
    let mut st = steps(b"xyz");
    let (d, k) = (Arc::clone(&dest), Arc::clone(&seen));
    st.at_filled = Some(Box::new(move || {
        *k.lock().unwrap() = Some((d.hidden("c.txt").unwrap(), d.born("c.txt").unwrap()));
    }));
    let Claim::Named(c) = s.fill_as("c.txt", &plan, &mut st) else {
        panic!("not named");
    };
    assert_eq!(
        *seen.lock().unwrap(),
        Some((true, Some(tag))),
        "while filled"
    );
    // Named, but still hidden and tagged until it is committed and then shown.
    assert!(
        dest.hidden("c.txt").unwrap(),
        "not shown before it is committed"
    );
    assert_eq!(dest.born("c.txt").unwrap(), Some(tag));
    let show = c
        .show()
        .expect("a filled-in file has a show to finish")
        .clone();
    c.keep();
    assert_eq!(
        dest.finish_show("c.txt", &show, ShowCheck::Made).unwrap(),
        Shown::Shown
    );
    assert!(!dest.hidden("c.txt").unwrap(), "shown once complete");
    // Asked again: already shown, nothing touched.
    assert_eq!(
        dest.finish_show("c.txt", &show, ShowCheck::Made).unwrap(),
        Shown::AlreadyShown
    );
    assert_eq!(
        dest.born("c.txt").unwrap(),
        Some(pctwin_gate::Born::of(real))
    );
}

/// Setting a modified time can move a creation time (APFS moves it back to the modified time):
/// the tag is never exposed to that while it is the proof. A file whose original is older than
/// any tag keeps its tag until it is recorded filled, and lands with the original's modified time
/// and a real creation date.
#[cfg(any(windows, target_os = "macos"))]
#[test]
fn an_old_original_never_costs_the_tag_and_never_leaves_it_behind() {
    let root = tempfile::tempdir().unwrap();
    let dest = Arc::new(fill_in(root.path()));
    let old = std::time::UNIX_EPOCH + std::time::Duration::from_secs(347_155_200); // 1981
    let mut file = dest
        .create_file(&IncomingPath::parse("old.txt").unwrap(), 3)
        .unwrap();
    file.write_all(b"old").unwrap();
    file.keep_modified_time(old);
    let s = common::seal(file).unwrap();
    let plan = s.fill_plan().unwrap();
    let tag = plan.tag.born.unwrap();
    let seen = Arc::new(std::sync::Mutex::new(None));
    let mut st = steps(b"old");
    let (d, k) = (Arc::clone(&dest), Arc::clone(&seen));
    st.at_filled = Some(Box::new(move || {
        *k.lock().unwrap() = d.born("old.txt").unwrap();
    }));
    let Claim::Named(c) = s.fill_as("old.txt", &plan, &mut st) else {
        panic!("not named");
    };
    let show = c.show().unwrap().clone();
    c.keep();
    assert_eq!(
        dest.finish_show("old.txt", &show, ShowCheck::Made).unwrap(),
        Shown::Shown
    );
    assert_eq!(
        *seen.lock().unwrap(),
        Some(tag),
        "the tag held until it was recorded filled"
    );
    let meta = std::fs::metadata(root.path().join("old.txt")).unwrap();
    assert_eq!(meta.modified().unwrap(), old);
    assert_ne!(
        dest.born("old.txt").unwrap(),
        Some(tag),
        "no tag left on the person's file"
    );
}

/// Linux can neither tag nor hide a file: a drive there whose identities change is never filled
/// in (nothing would prove such a file PCTwin's after a restart).
#[cfg(target_os = "linux")]
#[test]
fn a_drive_that_could_not_prove_a_filled_file_is_never_filled() {
    let root = tempfile::tempdir().unwrap();
    let dest = Destination::open(root.path())
        .unwrap()
        .with_profile(DriveProfile::of(DriveKind::ExFat).with_naming(Naming::FillIn));
    let s = sealed(&dest, "e.txt", b"e");
    assert!(matches!(s.fill_plan(), Err(GateError::NoSafeName)));
}

/// A name already taken (even by an empty file) is never touched: the file gets a numbered name.
#[test]
fn a_file_already_at_the_name_is_never_touched_and_a_clash_is_numbered() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("d.txt"), b"").unwrap();
    std::fs::write(root.path().join("d (2).txt"), b"theirs").unwrap();
    let dest = fill_in(root.path());
    let s = sealed(&dest, "d.txt", b"ours");
    let plan = s.fill_plan().unwrap();
    // Taken between finding it free and filling it, as another program would: left alone.
    let Claim::Taken(s) = s.fill_as("d.txt", &plan, &mut steps(b"ours")) else {
        panic!("a taken name was filled");
    };
    let Claim::Taken(s) = s.fill_as("d (2).txt", &plan, &mut steps(b"ours")) else {
        panic!("a taken name was filled");
    };
    let claimed = s.claim().unwrap();
    assert_eq!(claimed.finished().final_path, "d (3).txt");
    claimed.keep();
    assert_eq!(std::fs::read(root.path().join("d.txt")).unwrap(), b"");
    assert_eq!(
        std::fs::read(root.path().join("d (2).txt")).unwrap(),
        b"theirs"
    );
    assert_eq!(
        std::fs::read(root.path().join("d (3).txt")).unwrap(),
        b"ours"
    );
    assert_eq!(names(root.path()), ["d (2).txt", "d (3).txt", "d.txt"]);
}

/// Another program puts its own file at the name part way through the copy: PCTwin notices
/// before calling it filled, never touches that file, and keeps its own checked copy.
#[test]
fn another_program_replacing_the_file_mid_copy_is_noticed_and_its_file_kept() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().to_path_buf();
    let dest = fill_in(root.path()).with_fill_hook(1000, move |written| {
        if written == 1000 {
            std::fs::write(path.join("other.tmp"), b"their file").unwrap();
            replace(&path.join("other.tmp"), &path.join("e.bin"))?;
        }
        Ok(())
    });
    let bytes = vec![1u8; 3000];
    let s = sealed(&dest, "e.bin", &bytes);
    let temp = s.temp_path();
    let plan = s.fill_plan().unwrap();
    let mut st = steps(&bytes);
    let Claim::Failed(e, back) = s.fill_as("e.bin", &plan, &mut st) else {
        panic!("a replaced file was called filled");
    };
    assert!(matches!(e, GateError::ReplacedWhileFilling), "{e:?}");
    assert!(!st.log.contains(&"filled".to_string()), "{:?}", st.log);
    assert!(!st.log.contains(&"cleared".to_string()), "{:?}", st.log);
    assert_eq!(
        std::fs::read(root.path().join("e.bin")).unwrap(),
        b"their file"
    );
    assert_eq!(back.temp_path(), temp);
    back.persist();
    assert_eq!(std::fs::read(root.path().join(&temp)).unwrap(), bytes);
}

/// Moves `from` onto `to`, replacing it, as another program may.
fn replace(from: &Path, to: &Path) -> io::Result<()> {
    std::fs::rename(from, to)
}

/// A copy that fails part way removes PCTwin's own partial copy (proven: the very file, with
/// the bytes copied) and records that; the checked file comes back.
#[test]
fn a_copy_error_removes_only_pctwin_s_own_partial_copy() {
    let root = tempfile::tempdir().unwrap();
    let dest = fill_in(root.path()).with_fill_hook(1000, |written| {
        if written >= 2000 {
            Err(io::Error::other("the drive was pulled out"))
        } else {
            Ok(())
        }
    });
    let bytes = vec![2u8; 3000];
    let s = sealed(&dest, "f.bin", &bytes);
    let temp = s.temp_path();
    let plan = s.fill_plan().unwrap();
    let mut st = steps(&bytes);
    let Claim::Failed(e, back) = s.fill_as("f.bin", &plan, &mut st) else {
        panic!("a failed copy was named");
    };
    assert!(matches!(e, GateError::Io(_)), "{e:?}");
    assert_eq!(st.log, ["made", "cleared"]);
    assert!(!root.path().join("f.bin").exists());
    back.persist();
    assert_eq!(names(root.path()), [temp]);
}

/// The same failure after another program put its file at the name: that file stays.
#[test]
fn a_copy_error_never_removes_another_program_s_file() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().to_path_buf();
    let dest = fill_in(root.path()).with_fill_hook(1000, move |written| {
        if written == 2000 {
            std::fs::write(path.join("other.tmp"), b"their file").unwrap();
            replace(&path.join("other.tmp"), &path.join("g.bin"))?;
            return Err(io::Error::other("the drive was pulled out"));
        }
        Ok(())
    });
    let bytes = vec![3u8; 3000];
    let s = sealed(&dest, "g.bin", &bytes);
    let plan = s.fill_plan().unwrap();
    let mut st = steps(&bytes);
    let Claim::Failed(_, back) = s.fill_as("g.bin", &plan, &mut st) else {
        panic!("a failed copy was named");
    };
    assert_eq!(st.log, ["made"]);
    assert_eq!(
        std::fs::read(root.path().join("g.bin")).unwrap(),
        b"their file"
    );
    back.persist();
}

/// A copy that does not read back the same is never called filled, and is removed.
#[test]
fn a_copy_that_reads_back_differently_is_never_called_filled() {
    let root = tempfile::tempdir().unwrap();
    let dest = fill_in(root.path());
    let s = sealed(&dest, "h.bin", b"abc");
    let plan = s.fill_plan().unwrap();
    let mut st = steps(b"not what was written");
    let Claim::Failed(_, back) = s.fill_as("h.bin", &plan, &mut st) else {
        panic!("named");
    };
    assert_eq!(st.log, ["made", "read back", "cleared"]);
    assert!(!root.path().join("h.bin").exists());
    back.persist();
}

/// A drive that names files at once is never filled in, even if asked.
#[test]
fn only_a_drive_that_needs_it_is_filled_in() {
    let root = tempfile::tempdir().unwrap();
    let dest = Destination::open(root.path()).unwrap();
    let s = sealed(&dest, "i.txt", b"x");
    let plan = FillPlan {
        tag: pctwin_gate::FillTag {
            born: None,
            hidden: false,
        },
        proof: Proof::StableId,
    };
    let Claim::Failed(e, back) = s.fill_as("i.txt", &plan, &mut steps(b"x")) else {
        panic!("filled in on an ordinary drive");
    };
    assert!(matches!(e, GateError::NoSafeName), "{e:?}");
    assert!(!root.path().join("i.txt").exists());
    drop(back);
}

/// Many times over, PCTwin and another program make the same name at the same moment: exactly
/// one of them gets it, every time, and the other's file is never touched or mixed in.
#[test]
fn ten_thousand_races_for_a_name_never_mix_or_replace_a_file() {
    let rounds: u32 = std::env::var("PCTWIN_RACE_ROUNDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10_000);
    let root = tempfile::tempdir().unwrap();
    let dest = fill_in(root.path());
    let ours = b"ours".to_vec();
    let mut pending: Option<Sealed<'_>> = None;
    let (mut won, mut lost) = (0u32, 0u32);
    for round in 0..rounds {
        let name = format!("r{round}.bin");
        let s = match pending.take() {
            Some(s) => s,
            None => sealed(&dest, "x.bin", &ours),
        };
        let plan = s.fill_plan().unwrap();
        let theirs = root.path().join(&name);
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let b = Arc::clone(&barrier);
        let other = std::thread::spawn(move || {
            b.wait();
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&theirs)
            {
                Ok(mut f) => {
                    f.write_all(b"theirs").unwrap();
                    true
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => false,
                Err(e) => panic!("{e}"),
            }
        });
        barrier.wait();
        let claim = s.fill_as(&name, &plan, &mut steps(&ours));
        let they_won = other.join().unwrap();
        let got = std::fs::read(root.path().join(&name)).unwrap();
        match claim {
            Claim::Named(c) => {
                assert!(!they_won, "round {round}: both got the name");
                c.keep();
                assert_eq!(got, ours, "round {round}");
                won += 1;
            }
            Claim::Taken(back) => {
                assert!(they_won, "round {round}: nobody got the name");
                assert_eq!(got, b"theirs", "round {round}");
                pending = Some(back);
                lost += 1;
            }
            Claim::Failed(e, _) => panic!("round {round}: {e}"),
        }
    }
    assert_eq!(won + lost, rounds);
    eprintln!("PCTwin got the name {won} times, the other program {lost} times");
}

/// The journal's steps can fail too: one that cannot record the file made stops the fill, and
/// the file just made (proven PCTwin's: nothing copied yet) is removed.
#[test]
fn a_fill_that_cannot_be_recorded_stops_and_leaves_nothing_under_the_name() {
    struct NoJournal;
    impl FillSteps for NoJournal {
        fn made(&mut self, _: FileId) -> io::Result<()> {
            Err(io::Error::other("the journal is full"))
        }
        fn read_back(&mut self, _: &mut dyn Read) -> io::Result<bool> {
            Ok(true)
        }
        fn filled(&mut self) -> io::Result<()> {
            Ok(())
        }
        fn cleared(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let root = tempfile::tempdir().unwrap();
    let dest = fill_in(root.path());
    let s = sealed(&dest, "j.txt", b"abc");
    let plan = s.fill_plan().unwrap();
    let Claim::Failed(_, back) = s.fill_as("j.txt", &plan, &mut NoJournal) else {
        panic!("named without a journal");
    };
    assert!(!root.path().join("j.txt").exists());
    drop(back);
    assert!(names(root.path()).is_empty(), "{:?}", names(root.path()));
}

/// What a fill copies before the test's other program acts, and the tag PCTwin chose.
type Shared = Arc<std::sync::Mutex<Option<pctwin_gate::Born>>>;

/// Runs a fill of 3000 bytes in 1000-byte pieces where, after the first piece, `act` plays
/// another program on the folder and the drive then fails. Returns what the journal was told.
fn fail_after(
    name: &str,
    act: impl Fn(&Path, Option<pctwin_gate::Born>) + Send + Sync + 'static,
) -> (tempfile::TempDir, Vec<String>) {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().to_path_buf();
    let tag: Shared = Arc::default();
    let seen = Arc::clone(&tag);
    let dest = fill_in(root.path()).with_fill_hook(1000, move |written| {
        if written == 1000 {
            act(&path, *seen.lock().unwrap());
            return Err(io::Error::other("the drive was pulled out"));
        }
        Ok(())
    });
    let bytes = vec![4u8; 3000];
    let s = sealed(&dest, name, &bytes);
    let plan = s.fill_plan().unwrap();
    *tag.lock().unwrap() = plan.tag.born;
    let mut st = steps(&bytes);
    let Claim::Failed(_, back) = s.fill_as(name, &plan, &mut st) else {
        panic!("a failed copy was named");
    };
    back.persist();
    drop(dest);
    (root, st.log)
}

#[cfg(any(windows, target_os = "macos"))]
fn set_created(path: &Path, born: pctwin_gate::Born) {
    #[cfg(target_os = "macos")]
    use std::os::macos::fs::FileTimesExt;
    #[cfg(windows)]
    use std::os::windows::fs::FileTimesExt;
    let t = std::time::UNIX_EPOCH
        + std::time::Duration::new(u64::try_from(born.secs).unwrap(), born.nanos);
    let f = std::fs::OpenOptions::new().write(true).open(path).unwrap();
    f.set_times(std::fs::FileTimes::new().set_created(t))
        .unwrap();
}

/// Another program wrote more into PCTwin's file: it is no longer only PCTwin's copy, so a
/// failure leaves it.
#[test]
fn a_failure_leaves_pctwin_s_file_once_another_program_wrote_into_it() {
    let (root, log) = fail_after("m.bin", |dir, _| {
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(dir.join("m.bin"))
            .unwrap();
        f.write_all(b"theirs").unwrap();
    });
    assert_eq!(log, ["made"]);
    assert_eq!(
        std::fs::metadata(root.path().join("m.bin")).unwrap().len(),
        1006
    );
}

/// Another program's file put at the name, exactly as long as what PCTwin had copied and even
/// carrying PCTwin's tag: not the same file, so a failure leaves it.
#[test]
fn a_failure_never_removes_a_look_alike_file_put_at_the_name() {
    let (root, log) = fail_after("n.bin", |dir, tag| {
        let other = dir.join("other.tmp");
        std::fs::write(&other, vec![4u8; 1000]).unwrap();
        #[cfg(any(windows, target_os = "macos"))]
        if let Some(tag) = tag {
            set_created(&other, tag);
        }
        let _ = tag;
        replace(&other, &dir.join("n.bin")).unwrap();
    });
    assert_eq!(log, ["made"]);
    assert_eq!(
        std::fs::read(root.path().join("n.bin")).unwrap(),
        vec![4u8; 1000]
    );
}

/// PCTwin's file with its tag changed by another program: no longer provably PCTwin's (where a
/// drive's identities are not to be trusted, the tag is the proof), so a failure leaves it.
#[cfg(any(windows, target_os = "macos"))]
#[test]
fn a_failure_leaves_pctwin_s_file_once_its_tag_was_changed() {
    let (root, log) = fail_after("o.bin", |dir, _| {
        set_created(
            &dir.join("o.bin"),
            pctwin_gate::Born {
                secs: 1_700_000_000,
                nanos: 0,
            },
        );
    });
    assert_eq!(log, ["made"]);
    assert!(root.path().join("o.bin").exists());
}

/// Another program wrote into PCTwin's file, in place (same length, same tag): a failure then
/// leaves it, since PCTwin can remove only bytes it wrote.
#[test]
fn a_failure_leaves_a_file_another_program_wrote_into_in_place() {
    let (root, log) = fail_after("p.bin", |dir, _| {
        use std::io::{Seek, SeekFrom};
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .open(dir.join("p.bin"))
            .unwrap();
        f.seek(SeekFrom::Start(10)).unwrap();
        f.write_all(b"0123456789").unwrap();
    });
    assert_eq!(log, ["made"]);
    let kept = std::fs::read(root.path().join("p.bin")).unwrap();
    assert_eq!(kept.len(), 1000);
    assert_eq!(&kept[10..20], b"0123456789");
}

/// Another program puts its file at the name after the read-back and before it is recorded
/// filled: noticed by the last check, and its file kept.
#[test]
fn a_file_put_at_the_name_after_the_read_back_is_noticed() {
    let root = tempfile::tempdir().unwrap();
    let dest = fill_in(root.path());
    let bytes = vec![5u8; 3000];
    let s = sealed(&dest, "q.bin", &bytes);
    let plan = s.fill_plan().unwrap();
    let dir = root.path().to_path_buf();
    let mut st = steps(&bytes);
    st.after_read = Some(Box::new(move || {
        std::fs::write(dir.join("other.tmp"), b"their file").unwrap();
        replace(&dir.join("other.tmp"), &dir.join("q.bin")).unwrap();
    }));
    let Claim::Failed(e, back) = s.fill_as("q.bin", &plan, &mut st) else {
        panic!("a replaced file was called filled");
    };
    assert!(matches!(e, GateError::ReplacedWhileFilling), "{e:?}");
    assert!(!st.log.iter().any(|l| l == "filled"), "{:?}", st.log);
    assert_eq!(
        std::fs::read(root.path().join("q.bin")).unwrap(),
        b"their file"
    );
    back.persist();
}

/// After a restart, the gate proves a filled-in file PCTwin's again before it finishes or
/// removes it, by the rule the journal recorded: the recorded identity where identities last, or
/// the tag; and (to remove one) bytes that are exactly the start of the checked file. Anything
/// else is left exactly as it is.
#[test]
fn after_a_restart_only_a_file_proven_pctwin_s_is_finished_or_removed() {
    use pctwin_gate::{Born, Fill, FillFinish, FillRemoval, FillTag};
    let root = tempfile::tempdir().unwrap();
    let dest = Destination::open(root.path()).unwrap();
    std::fs::write(root.path().join(".pctwin-t.part"), b"abcdefgh").unwrap();
    let at = |name: &str, bytes: &[u8]| {
        std::fs::write(root.path().join(name), bytes).unwrap();
        let id = dest.stat(name).unwrap().unwrap().id;
        (id, dest.born(name).unwrap())
    };
    let fill = |proof, made: Option<FileId>, born: Option<Born>| Fill {
        final_path: "x".into(),
        made,
        tag: FillTag {
            born,
            hidden: false,
        },
        proof,
    };
    let other_born = Born {
        secs: 1_000_000_000,
        nanos: 0,
    };
    let at_name = |f: &Fill, name: &str| Fill {
        final_path: name.into(),
        ..f.clone()
    };
    // Proven by nothing that lasts: identity good only while held, or none at all.
    for proof in [Proof::SameMountId, Proof::None] {
        let (id, born) = at("n.bin", b"abcd");
        let f = at_name(&fill(proof, Some(id), born), "n.bin");
        assert_eq!(
            dest.remove_fill("n.bin", &f, ".pctwin-t.part", 4).unwrap(),
            FillRemoval::NotProven,
            "{proof:?}"
        );
        let r = dest.finish_fill("n.bin", &f, |_| Ok(true), || Ok(()));
        assert!(matches!(r, Ok(FillFinish::NotProven)), "{proof:?}: {r:?}");
        assert!(root.path().join("n.bin").exists());
    }
    // A tag that is not the file's.
    let (_, _) = at("t.bin", b"abcd");
    let f = at_name(&fill(Proof::Token, None, Some(other_born)), "t.bin");
    assert_eq!(
        dest.remove_fill("t.bin", &f, ".pctwin-t.part", 4).unwrap(),
        FillRemoval::NotProven
    );
    // PCTwin's by its tag, but its bytes are not the start of the checked file.
    let (_, born) = at("b.bin", b"abXd");
    if born.is_some() {
        let f = at_name(&fill(Proof::Token, None, born), "b.bin");
        assert_eq!(
            dest.remove_fill("b.bin", &f, ".pctwin-t.part", 4).unwrap(),
            FillRemoval::NotProven
        );
        assert!(root.path().join("b.bin").exists());
    }
    // PCTwin's by its recorded identity, and exactly the start of the checked file: removed.
    let (id, _) = at("ok.bin", b"abcd");
    let f = at_name(&fill(Proof::StableId, Some(id), None), "ok.bin");
    assert_eq!(
        dest.remove_fill("ok.bin", &f, ".pctwin-t.part", 4).unwrap(),
        FillRemoval::Removed
    );
    assert!(!root.path().join("ok.bin").exists());
}

/// Showing a committed, filled-in file never touches one PCTwin cannot prove its own: a hidden
/// file whose tag is not the recorded one is left exactly as it is; a name with nothing at it is
/// gone; and each step of a show can be done again after a stop part way.
#[cfg(any(windows, target_os = "macos"))]
#[test]
fn showing_a_committed_file_touches_only_one_proven_pctwin_s() {
    let root = tempfile::tempdir().unwrap();
    let steps_seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let seen = Arc::clone(&steps_seen);
    let dest = fill_in(root.path()).with_show_hook(move |step| {
        seen.lock().unwrap().push(step.to_string());
        Ok(())
    });
    let s = sealed(&dest, "s.txt", b"show");
    let plan = s.fill_plan().unwrap();
    let Claim::Named(c) = s.fill_as("s.txt", &plan, &mut steps(b"show")) else {
        panic!("not named");
    };
    let show = c.show().unwrap().clone();
    c.keep();
    // Not proven: the recorded tag is not this file's.
    let mut other = show.clone();
    other.fill.tag.born = Some(pctwin_gate::Born {
        secs: 1_000_000_000,
        nanos: 0,
    });
    other.fill.proof = Proof::Token;
    assert_eq!(
        dest.finish_show("s.txt", &other, ShowCheck::Made).unwrap(),
        Shown::NotProven
    );
    assert!(dest.hidden("s.txt").unwrap(), "left exactly as it is");
    assert!(steps_seen.lock().unwrap().is_empty());
    // Proven: shown in order, the creation date given back last.
    assert_eq!(
        dest.finish_show("s.txt", &show, ShowCheck::Made).unwrap(),
        Shown::Shown
    );
    assert_eq!(
        *steps_seen.lock().unwrap(),
        ["shown", "modified", "created"]
    );
    // Nothing at a name: gone.
    let mut gone = show.clone();
    gone.fill.final_path = "nothing.txt".into();
    assert_eq!(
        dest.finish_show("nothing.txt", &gone, ShowCheck::Made)
            .unwrap(),
        Shown::Gone
    );
}

/// After a restart, an empty file made hidden just before its tag (nothing recorded as made) is
/// PCTwin's own and removed; an empty file that is not hidden is not.
#[cfg(windows)]
#[test]
fn after_a_restart_an_empty_hidden_file_made_before_its_tag_is_removed() {
    use pctwin_gate::{Fill, FillRemoval, FillTag};
    use std::os::windows::fs::OpenOptionsExt;
    let root = tempfile::tempdir().unwrap();
    let dest = Destination::open(root.path()).unwrap();
    std::fs::write(root.path().join(".pctwin-t.part"), b"abc").unwrap();
    let fill = |name: &str| Fill {
        final_path: name.into(),
        made: None,
        tag: FillTag {
            born: Some(pctwin_gate::Born {
                secs: 1_000_000_000,
                nanos: 0,
            }),
            hidden: true,
        },
        proof: Proof::Token,
    };
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .attributes(0x2)
        .open(root.path().join("h.bin"))
        .unwrap();
    assert_eq!(
        dest.remove_fill("h.bin", &fill("h.bin"), ".pctwin-t.part", 0)
            .unwrap(),
        FillRemoval::Removed
    );
    std::fs::write(root.path().join("v.bin"), b"").unwrap();
    assert_eq!(
        dest.remove_fill("v.bin", &fill("v.bin"), ".pctwin-t.part", 0)
            .unwrap(),
        FillRemoval::NotProven
    );
    assert!(root.path().join("v.bin").exists());
    let hidden = |name: &str, bytes: &[u8]| {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .attributes(0x2)
            .open(root.path().join(name))
            .unwrap()
            .write_all(bytes)
            .unwrap();
    };
    // Not empty: something was written into it, so it is not the file made just before its tag,
    // even when its bytes are the start of PCTwin's.
    hidden("p.bin", b"abc");
    assert_eq!(
        dest.remove_fill("p.bin", &fill("p.bin"), ".pctwin-t.part", 3)
            .unwrap(),
        FillRemoval::NotProven
    );
    assert!(root.path().join("p.bin").exists());
    // Something was recorded as made, and this is not it: the tag came before that, so an
    // untagged file here is not PCTwin's.
    hidden("m.bin", b"");
    let mut made = fill("m.bin");
    made.made = Some(FileId {
        volume: 7,
        index: std::num::NonZeroU64::new(7).unwrap(),
        born: None,
    });
    assert_eq!(
        dest.remove_fill("m.bin", &made, ".pctwin-t.part", 0)
            .unwrap(),
        FillRemoval::NotProven
    );
    assert!(root.path().join("m.bin").exists());
}

/// Windows gives a file made at a name just freed the creation time of the file that had it
/// (file-system tunnelling), so another program's file put in place of PCTwin's hidden one
/// carries PCTwin's tag. It is still never shown: its identity is not the one made, and its bytes
/// are not the file's.
#[cfg(windows)]
#[test]
fn a_file_put_in_place_of_pctwin_s_hidden_one_is_never_shown_though_it_carries_the_tag() {
    use std::os::windows::fs::OpenOptionsExt;
    let root = tempfile::tempdir().unwrap();
    let dest = fill_in(root.path());
    let s = sealed(&dest, "u.txt", b"ours, whole");
    let plan = s.fill_plan().unwrap();
    let Claim::Named(c) = s.fill_as("u.txt", &plan, &mut steps(b"ours, whole")) else {
        panic!("not named");
    };
    let show = c.show().unwrap().clone();
    c.keep();
    std::fs::remove_file(root.path().join("u.txt")).unwrap();
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .attributes(0x2)
        .open(root.path().join("u.txt"))
        .unwrap()
        .write_all(b"theirs")
        .unwrap();
    // Tunnelling: their file has PCTwin's tag.
    assert_eq!(dest.born("u.txt").unwrap(), show.fill.tag.born);
    assert_eq!(
        dest.finish_show("u.txt", &show, ShowCheck::Made).unwrap(),
        Shown::NotProven
    );
    let mut whole = |f: &mut dyn Read| {
        let mut got = Vec::new();
        f.read_to_end(&mut got)?;
        Ok(got == b"ours, whole")
    };
    assert_eq!(
        dest.finish_show("u.txt", &show, ShowCheck::Bytes(&mut whole))
            .unwrap(),
        Shown::NotProven
    );
    assert!(dest.hidden("u.txt").unwrap(), "their file is left as it is");
    assert_eq!(std::fs::read(root.path().join("u.txt")).unwrap(), b"theirs");
}
