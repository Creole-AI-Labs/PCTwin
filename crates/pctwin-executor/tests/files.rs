//! Putting one settings file back only if it is still PCTwin's version, on the real file system
//! (security review finding 2; T23-017, T23-018, T23-019): a save the person makes at any moment
//! between the steps is never lost, and a crash at any moment is finished by running it again.

use std::path::Path;

use pctwin_executor::files::{Moment, Swap, Swapped, private_name, replace_if_unchanged};

const OURS: &[u8] = b"PCTwin's copy from the old laptop";
const EARLIER: &[u8] = b"the new laptop's own settings from before the move";
const PERSONS: &[u8] = b"the person's new edit";

fn hash(b: &[u8]) -> [u8; 32] {
    *blake3::hash(b).as_bytes()
}

fn names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

fn put(dir: &Path, name: &str, bytes: &[u8]) {
    std::fs::write(dir.join(name), bytes).unwrap();
}

fn swap<'a>(private: &'a str, earlier: Option<&'a [u8]>) -> Swap<'a> {
    Swap {
        private,
        ours: hash(OURS),
        earlier,
    }
}

#[test]
fn an_unchanged_file_is_put_back() {
    for earlier in [Some(EARLIER), None] {
        let dir = tempfile::tempdir().unwrap();
        put(dir.path(), "vlcrc", OURS);
        let p = private_name().unwrap();
        let got =
            replace_if_unchanged(dir.path(), "vlcrc", &swap(&p, earlier), &mut |_| {}).unwrap();
        assert_eq!(got, Swapped::Restored);
        match earlier {
            Some(b) => {
                assert_eq!(std::fs::read(dir.path().join("vlcrc")).unwrap(), b);
                assert_eq!(names(dir.path()), ["vlcrc"]);
            }
            None => assert!(names(dir.path()).is_empty()),
        }
    }
}

#[test]
fn a_file_the_person_changed_is_kept_untouched() {
    let dir = tempfile::tempdir().unwrap();
    put(dir.path(), "vlcrc", PERSONS);
    let p = private_name().unwrap();
    let got =
        replace_if_unchanged(dir.path(), "vlcrc", &swap(&p, Some(EARLIER)), &mut |_| {}).unwrap();
    assert_eq!(got, Swapped::Kept);
    assert_eq!(std::fs::read(dir.path().join("vlcrc")).unwrap(), PERSONS);
    assert_eq!(names(dir.path()), ["vlcrc"]);
}

#[test]
fn nothing_there_is_nothing_to_put_back() {
    let dir = tempfile::tempdir().unwrap();
    let p = private_name().unwrap();
    let got =
        replace_if_unchanged(dir.path(), "vlcrc", &swap(&p, Some(EARLIER)), &mut |_| {}).unwrap();
    assert_eq!(got, Swapped::Gone);
    assert!(names(dir.path()).is_empty());
}

/// Review proof P2 on the real file system: the person saves (an app writing a new file under the
/// name, as editors do) at every moment between the steps; their save is always kept.
#[test]
fn a_save_at_any_moment_is_never_lost() {
    for at in [Moment::Aside, Moment::Checked, Moment::Written] {
        let dir = tempfile::tempdir().unwrap();
        put(dir.path(), "vlcrc", OURS);
        let p = private_name().unwrap();
        let d = dir.path().to_path_buf();
        let got = replace_if_unchanged(dir.path(), "vlcrc", &swap(&p, Some(EARLIER)), &mut |m| {
            if m == at {
                put(&d, "vlcrc", PERSONS);
            }
        })
        .unwrap();
        assert_eq!(got, Swapped::Kept, "{at:?}");
        assert_eq!(
            std::fs::read(dir.path().join("vlcrc")).unwrap(),
            PERSONS,
            "{at:?}"
        );
        // Nothing of PCTwin's is left behind, and nothing of the person's was moved away.
        assert_eq!(names(dir.path()), ["vlcrc"], "{at:?}");
    }
}

/// A program that already had the file open writes into it after it was moved aside: on Linux
/// and macOS that write lands in the moved copy, which is then not PCTwin's version, so it is put
/// back; on Windows the moved copy is held so nobody can write it.
#[test]
fn a_write_into_the_moved_copy_is_kept() {
    let dir = tempfile::tempdir().unwrap();
    put(dir.path(), "vlcrc", OURS);
    let p = private_name().unwrap();
    let d = dir.path().to_path_buf();
    let private = p.clone();
    let got = replace_if_unchanged(dir.path(), "vlcrc", &swap(&p, Some(EARLIER)), &mut |m| {
        if m == Moment::Aside {
            let w = std::fs::OpenOptions::new()
                .write(true)
                .open(d.join(&private));
            #[cfg(windows)]
            assert!(w.is_err(), "held: nobody can write it");
            #[cfg(unix)]
            {
                use std::io::Write;
                w.unwrap().write_all(b"!").unwrap();
            }
        }
    })
    .unwrap();
    #[cfg(unix)]
    {
        assert_eq!(got, Swapped::Kept);
        let mut want = OURS.to_vec();
        want[0] = b'!';
        assert_eq!(std::fs::read(dir.path().join("vlcrc")).unwrap(), want);
    }
    #[cfg(windows)]
    {
        assert_eq!(got, Swapped::Restored);
        assert_eq!(std::fs::read(dir.path().join("vlcrc")).unwrap(), EARLIER);
    }
    assert_eq!(names(dir.path()), ["vlcrc"]);
}

/// A crash at any moment (the step stops there) is finished by running it again with the same
/// private name, as undo does from the journal.
#[test]
fn a_crash_at_any_moment_is_finished_by_running_again() {
    for at in [Moment::Aside, Moment::Checked, Moment::Written] {
        for persons in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            put(dir.path(), "vlcrc", if persons { PERSONS } else { OURS });
            let p = private_name().unwrap();
            let s = swap(&p, Some(EARLIER));
            let stopped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                replace_if_unchanged(dir.path(), "vlcrc", &s, &mut |m| {
                    if m == at {
                        panic!("the laptop lost power");
                    }
                })
            }));
            let first = stopped.ok().map(Result::unwrap);
            let got = match first {
                Some(done) => done,
                None => replace_if_unchanged(dir.path(), "vlcrc", &s, &mut |_| {}).unwrap(),
            };
            let (want, bytes) = if persons {
                (Swapped::Kept, PERSONS)
            } else {
                (Swapped::Restored, EARLIER)
            };
            assert_eq!(got, want, "{at:?} {persons}");
            assert_eq!(std::fs::read(dir.path().join("vlcrc")).unwrap(), bytes);
            assert_eq!(names(dir.path()), ["vlcrc"], "{at:?} {persons}");
        }
    }
}

/// A file that is not the person's and took the name while the person's own file was aside is
/// never written over: the person's file goes beside it.
#[test]
fn the_persons_file_goes_beside_a_new_one_never_over_it() {
    let dir = tempfile::tempdir().unwrap();
    put(dir.path(), "vlcrc", PERSONS);
    let p = private_name().unwrap();
    let d = dir.path().to_path_buf();
    let got = replace_if_unchanged(dir.path(), "vlcrc", &swap(&p, Some(EARLIER)), &mut |m| {
        if m == Moment::Aside {
            put(&d, "vlcrc", b"an app's new file");
        }
    })
    .unwrap();
    assert_eq!(got, Swapped::Kept);
    assert_eq!(
        std::fs::read(dir.path().join("vlcrc")).unwrap(),
        b"an app's new file"
    );
    assert_eq!(
        std::fs::read(dir.path().join("vlcrc (kept by PCTwin)")).unwrap(),
        PERSONS
    );
}

#[test]
fn only_plain_names_and_private_names_are_used() {
    let dir = tempfile::tempdir().unwrap();
    let p = private_name().unwrap();
    assert!(pctwin_executor::files::is_private_name(&p));
    for bad in ["", "..", "a/b", "a\\b"] {
        assert!(replace_if_unchanged(dir.path(), bad, &swap(&p, None), &mut |_| {}).is_err());
    }
    for bad in ["vlcrc", ".pctwin-undo-zz", ".pctwin-undo-0123"] {
        assert!(replace_if_unchanged(dir.path(), "vlcrc", &swap(bad, None), &mut |_| {}).is_err());
    }
}

/// Windows: if another file takes the name between holding the file and moving it aside, the
/// file moved is not the one held: it is put back and nothing is replaced.
#[cfg(windows)]
#[test]
fn a_file_swapped_in_while_held_is_put_back() {
    let dir = tempfile::tempdir().unwrap();
    put(dir.path(), "vlcrc", OURS);
    let p = private_name().unwrap();
    let d = dir.path().to_path_buf();
    let got = replace_if_unchanged(dir.path(), "vlcrc", &swap(&p, Some(EARLIER)), &mut |m| {
        if m == Moment::Held {
            std::fs::rename(d.join("vlcrc"), d.join("elsewhere")).unwrap();
            put(&d, "vlcrc", PERSONS);
        }
    });
    assert!(got.is_err(), "{got:?}");
    assert_eq!(std::fs::read(dir.path().join("vlcrc")).unwrap(), PERSONS);
    assert_eq!(names(dir.path()), ["elsewhere", "vlcrc"]);
}

/// Linux and macOS: a write into the moved copy after it was first checked is caught by the
/// check just before PCTwin's version is removed, and that copy is kept.
#[cfg(unix)]
#[test]
fn a_late_write_into_the_moved_copy_is_kept() {
    use std::io::Write;
    let dir = tempfile::tempdir().unwrap();
    put(dir.path(), "vlcrc", OURS);
    let p = private_name().unwrap();
    let d = dir.path().to_path_buf();
    let private = p.clone();
    let got = replace_if_unchanged(dir.path(), "vlcrc", &swap(&p, Some(EARLIER)), &mut |m| {
        if m == Moment::Written {
            let mut w = std::fs::OpenOptions::new()
                .append(true)
                .open(d.join(&private))
                .unwrap();
            w.write_all(b" and more").unwrap();
        }
    })
    .unwrap();
    assert_eq!(got, Swapped::Kept);
    // The earlier version is under the name; the written-to copy is kept beside it.
    assert_eq!(std::fs::read(dir.path().join("vlcrc")).unwrap(), EARLIER);
    assert_eq!(
        std::fs::read(dir.path().join("vlcrc (kept by PCTwin)")).unwrap(),
        [OURS, b" and more"].concat()
    );
}
