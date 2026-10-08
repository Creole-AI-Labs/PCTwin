//! What the change journal needs from the gate (Task List 1.6): a temporary file whose name the
//! journal knows before it exists, a finish split into steps the journal can record between
//! (sealed: every byte on disk; the real name found and recorded first, then claimed; kept: done),
//! and, after a restart, giving a sealed temporary file its real name without ever overwriting,
//! telling one file from another by identity, and touching nothing but PCTwin's own temporary
//! files.

use std::io::Write;

use pctwin_gate::{Destination, GateError, IncomingPath, NameChange, temp_name};

fn landed(dest: &Destination, sent: &str, temp: &str) -> String {
    let sealed = dest.reopen_sealed(&path(sent), temp).unwrap();
    let name = sealed.next_name().unwrap();
    match sealed.claim_as(&name).unwrap() {
        Ok(claimed) => claimed.keep().final_path,
        Err(_) => panic!("taken"),
    }
}

fn path(s: &str) -> IncomingPath {
    IncomingPath::parse(s).unwrap()
}

fn setup() -> (tempfile::TempDir, Destination) {
    let root = tempfile::tempdir().unwrap();
    let dest = Destination::open(root.path()).unwrap();
    (root, dest)
}

fn names(dir: &std::path::Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

#[test]
fn a_tagged_file_is_written_under_the_temporary_name_the_journal_chose() {
    let (root, dest) = setup();
    assert_eq!(temp_name("00ff-7"), ".pctwin-00ff-7.part");
    let mut file = dest
        .create_file_tagged(&path("Docs/Tax/a.txt"), 3, "00ff-7")
        .unwrap();
    assert_eq!(file.temp_path(), "Docs/Tax/.pctwin-00ff-7.part");
    file.write_all(b"abc").unwrap();
    assert!(root.path().join("Docs/Tax/.pctwin-00ff-7.part").is_file());
    let done = file.finish().unwrap();
    assert_eq!(done.final_path, "Docs/Tax/a.txt");
    assert_eq!(names(&root.path().join("Docs/Tax")), ["a.txt"]);
}

#[test]
fn a_tag_must_be_plain_and_a_temporary_name_is_never_reused() {
    let (root, dest) = setup();
    for bad in ["", "A", "a/b", "..", "a.b", "a b", &"a".repeat(65)] {
        assert!(
            dest.create_file_tagged(&path("a.txt"), 1, bad).is_err(),
            "{bad:?}"
        );
    }
    assert!(
        dest.create_file_tagged(&path("a.txt"), 1, &"a".repeat(64))
            .is_ok()
    );
    // Left over from before: refused, and left exactly as it was.
    std::fs::write(root.path().join(".pctwin-1-1.part"), b"old").unwrap();
    assert!(matches!(
        dest.create_file_tagged(&path("a.txt"), 1, "1-1"),
        Err(GateError::Io(_))
    ));
    assert_eq!(
        std::fs::read(root.path().join(".pctwin-1-1.part")).unwrap(),
        b"old"
    );
}

#[test]
fn sealing_checks_every_byte_arrived_and_an_unsealed_file_leaves_nothing() {
    let (root, dest) = setup();
    let mut file = dest.create_file_tagged(&path("a.txt"), 4, "t-1").unwrap();
    file.write_all(b"abc").unwrap();
    assert!(matches!(
        file.seal(),
        Err(GateError::SizeMismatch {
            announced: 4,
            received: 3
        })
    ));
    assert!(names(root.path()).is_empty());
    let mut file = dest.create_file_tagged(&path("b.txt"), 1, "t-2").unwrap();
    file.write_all(b"b").unwrap();
    let sealed = file.seal().unwrap();
    assert_eq!(sealed.temp_path(), ".pctwin-t-2.part");
    assert!(root.path().join(".pctwin-t-2.part").is_file());
    drop(sealed);
    assert!(names(root.path()).is_empty());
}

#[test]
fn the_real_name_is_found_first_then_claimed_and_both_names_stay_until_kept() {
    let (root, dest) = setup();
    let mut file = dest.create_file_tagged(&path("d/a.txt"), 2, "t-1").unwrap();
    file.write_all(b"hi").unwrap();
    let sealed = file.seal().unwrap();
    // Found, not taken: the journal records it before the file gets it.
    assert_eq!(sealed.next_name().unwrap(), "d/a.txt");
    assert!(!root.path().join("d/a.txt").exists());
    let Ok(claimed) = sealed.claim_as("d/a.txt").unwrap() else {
        panic!("taken")
    };
    assert_eq!(claimed.finished().final_path, "d/a.txt");
    assert_eq!(claimed.finished().sent_path, "d/a.txt");
    assert_eq!(std::fs::read(root.path().join("d/a.txt")).unwrap(), b"hi");
    // Both names stay until the journal no longer needs the temporary one (drives with hard
    // links), so a crash in between can still be traced back to this write.
    assert!(root.path().join("d/.pctwin-t-1.part").exists());
    let done = claimed.keep();
    assert_eq!(done.final_path, "d/a.txt");
    assert_eq!(names(&root.path().join("d")), ["a.txt"]);
}

#[test]
fn a_name_taken_after_it_was_found_is_never_replaced_and_the_file_comes_back() {
    let (root, dest) = setup();
    let mut file = dest.create_file_tagged(&path("a.txt"), 6, "t-1").unwrap();
    file.write_all(b"theirs").unwrap();
    let sealed = file.seal().unwrap();
    let name = sealed.next_name().unwrap();
    assert_eq!(name, "a.txt");
    // Another program takes the name in between.
    std::fs::write(root.path().join("a.txt"), b"mine").unwrap();
    let Err(sealed) = sealed.claim_as(&name).unwrap() else {
        panic!("replaced a file")
    };
    assert_eq!(std::fs::read(root.path().join("a.txt")).unwrap(), b"mine");
    let name = sealed.next_name().unwrap();
    assert_eq!(name, "a (2).txt");
    let Ok(claimed) = sealed.claim_as(&name).unwrap() else {
        panic!("taken")
    };
    assert!(claimed.finished().changes.contains(&NameChange::NameClash));
    claimed.keep();
    assert_eq!(names(root.path()), ["a (2).txt", "a.txt"]);
    assert_eq!(
        std::fs::read(root.path().join("a (2).txt")).unwrap(),
        b"theirs"
    );
}

#[test]
fn a_file_is_only_ever_named_inside_its_own_folder() {
    let (root, dest) = setup();
    for elsewhere in ["a.txt", "e/a.txt", "d/../a.txt", "d/x/a.txt", "d/", ""] {
        let mut file = dest.create_file_tagged(&path("d/a.txt"), 1, "t-1").unwrap();
        file.write_all(b"x").unwrap();
        let sealed = file.seal().unwrap();
        assert!(
            sealed.claim_as(elsewhere).is_err(),
            "named outside its folder: {elsewhere:?}"
        );
        // Refused: the unnamed file is removed, nothing else appears.
        assert!(names(&root.path().join("d")).is_empty(), "{elsewhere:?}");
        assert_eq!(names(root.path()), ["d"], "{elsewhere:?}");
    }
}

#[test]
fn a_claimed_file_that_is_not_kept_stays_under_its_name_for_the_journal() {
    let (root, dest) = setup();
    let mut file = dest.create_file_tagged(&path("a.txt"), 1, "t-1").unwrap();
    file.write_all(b"x").unwrap();
    let claimed = file.seal().unwrap().claim().unwrap();
    drop(claimed);
    // The journal already has its name: recovery finishes it, never a silent removal.
    assert!(root.path().join("a.txt").is_file());
}

#[test]
fn a_sealed_file_left_by_a_crash_gets_its_real_name_without_overwriting() {
    let (root, dest) = setup();
    std::fs::create_dir(root.path().join("Docs")).unwrap();
    std::fs::write(root.path().join("Docs/.pctwin-t-9.part"), b"new").unwrap();
    std::fs::write(root.path().join("Docs/a.txt"), b"mine").unwrap();
    assert_eq!(
        landed(&dest, "Docs/a.txt", "Docs/.pctwin-t-9.part"),
        "Docs/a (2).txt"
    );
    assert_eq!(names(&root.path().join("Docs")), ["a (2).txt", "a.txt"]);
    assert_eq!(
        std::fs::read(root.path().join("Docs/a (2).txt")).unwrap(),
        b"new"
    );
    assert_eq!(
        std::fs::read(root.path().join("Docs/a.txt")).unwrap(),
        b"mine"
    );
}

#[test]
fn reopening_refuses_anything_but_a_pctwin_temporary_file() {
    let (root, dest) = setup();
    std::fs::write(root.path().join("a.txt"), b"mine").unwrap();
    std::fs::create_dir(root.path().join(".pctwin-t-1.part")).unwrap();
    for stored in [
        "a.txt",
        ".pctwin-t-1.part",
        ".pctwin-t-2.part",
        "../.pctwin-t-1.part",
        "gone/.pctwin-t-3.part",
        "",
    ] {
        assert!(
            dest.reopen_sealed(&path("b.txt"), stored).is_err(),
            "{stored:?}"
        );
    }
    assert_eq!(std::fs::read(root.path().join("a.txt")).unwrap(), b"mine");
    assert!(!root.path().join("b.txt").exists());
}

#[test]
fn an_empty_file_at_the_name_is_someone_else_s_and_is_never_replaced() {
    let (root, dest) = setup();
    std::fs::write(root.path().join(".pctwin-t-1.part"), b"data").unwrap();
    std::fs::write(root.path().join("a.txt"), b"").unwrap();
    let sealed = dest
        .reopen_sealed(&path("a.txt"), ".pctwin-t-1.part")
        .unwrap();
    let Err(sealed) = sealed.claim_as("a.txt").unwrap() else {
        panic!("replaced an empty file")
    };
    assert_eq!(std::fs::read(root.path().join("a.txt")).unwrap(), b"");
    assert_eq!(sealed.next_name().unwrap(), "a (2).txt");
}

#[test]
fn a_sealed_file_has_one_identity_until_it_lands_and_keeps_it_under_its_name() {
    let (root, dest) = setup();
    let mut file = dest.create_file_tagged(&path("a.txt"), 2, "t-1").unwrap();
    file.write_all(b"hi").unwrap();
    let sealed = file.seal().unwrap();
    let id = sealed.identity().unwrap();
    assert_eq!(dest.stat(".pctwin-t-1.part").unwrap().unwrap().id, id);
    let Ok(claimed) = sealed.claim_as("a.txt").unwrap() else {
        panic!("taken")
    };
    claimed.keep();
    assert_eq!(dest.stat("a.txt").unwrap().unwrap().id, id);
    assert!(!root.path().join(".pctwin-t-1.part").exists());
}

#[test]
fn a_file_has_one_identity_by_any_name_and_folders_have_theirs() {
    let (root, dest) = setup();
    std::fs::create_dir(root.path().join("d")).unwrap();
    std::fs::write(root.path().join("d/a.txt"), b"same").unwrap();
    std::fs::write(root.path().join("d/b.txt"), b"same").unwrap();
    std::fs::hard_link(root.path().join("d/a.txt"), root.path().join("d/c.txt")).unwrap();
    let a = dest.stat("d/a.txt").unwrap().unwrap();
    let b = dest.stat("d/b.txt").unwrap().unwrap();
    let c = dest.stat("d/c.txt").unwrap().unwrap();
    assert_eq!(a.len, 4);
    assert!(a.modified.is_some());
    assert_eq!(a.id, c.id);
    assert_ne!(a.id, b.id, "same contents, different file");
    assert_eq!(dest.stat("d/x.txt").unwrap(), None);
    assert_eq!(dest.stat("d").unwrap(), None);
    let d = dest.folder_identity("d").unwrap().unwrap();
    assert_ne!(d, a.id);
    assert!(dest.folder_identity("").unwrap().is_some());
    assert_eq!(dest.folder_identity("e").unwrap(), None);
    assert_eq!(dest.folder_identity("d/a.txt").unwrap(), None);
}

#[test]
fn only_folders_made_for_a_file_are_listed_as_made() {
    let (root, dest) = setup();
    std::fs::create_dir(root.path().join("mine")).unwrap();
    let file = dest
        .create_file_tagged(&path("mine/new/deeper/a.txt"), 1, "t-1")
        .unwrap();
    assert_eq!(file.created_folders(), ["mine/new", "mine/new/deeper"]);
    let other = dest
        .create_file_tagged(&path("mine/new/b.txt"), 1, "t-2")
        .unwrap();
    assert!(other.created_folders().is_empty());
}

#[test]
fn looking_finds_only_regular_files_and_never_creates_anything() {
    let (root, dest) = setup();
    std::fs::create_dir(root.path().join("d")).unwrap();
    std::fs::write(root.path().join("d/a.txt"), b"12345").unwrap();
    assert_eq!(dest.look("d/a.txt").unwrap(), Some(5));
    assert_eq!(dest.look("d/b.txt").unwrap(), None);
    assert_eq!(dest.look("e/b.txt").unwrap(), None);
    assert_eq!(dest.look("d").unwrap(), None);
    assert!(dest.look("../d/a.txt").is_err());
    // Only plain stored paths, even ones that would stay inside.
    assert!(dest.look("d/../d/a.txt").is_err());
    assert!(dest.look("d/./a.txt").is_err());
    assert!(dest.look("d//a.txt").is_err());
    assert_eq!(names(root.path()), ["d"]);
}

#[test]
fn removing_a_temporary_file_never_touches_anything_else() {
    let (root, dest) = setup();
    std::fs::create_dir(root.path().join("d")).unwrap();
    std::fs::write(root.path().join("d/.pctwin-t-1.part"), b"x").unwrap();
    std::fs::write(root.path().join("d/a.txt"), b"mine").unwrap();
    std::fs::create_dir(root.path().join("d/.pctwin-t-2.part")).unwrap();
    assert!(dest.remove_temp("d/.pctwin-t-1.part").unwrap());
    assert!(!dest.remove_temp("d/.pctwin-t-1.part").unwrap());
    assert!(!dest.remove_temp("gone/.pctwin-t-1.part").unwrap());
    assert!(dest.remove_temp("d/a.txt").is_err());
    // Looks like one, but the part between is not a tag PCTwin gives.
    std::fs::write(root.path().join("d/.pctwin-My Notes.part"), b"mine").unwrap();
    assert!(dest.remove_temp("d/.pctwin-My Notes.part").is_err());
    assert!(dest.remove_temp("d/.pctwin-.part").is_err());
    std::fs::remove_file(root.path().join("d/.pctwin-My Notes.part")).unwrap();
    assert!(!dest.remove_temp("d/.pctwin-t-2.part").unwrap());
    assert!(dest.remove_temp("../.pctwin-t-1.part").is_err());
    assert_eq!(names(&root.path().join("d")), [".pctwin-t-2.part", "a.txt"]);
}

#[test]
fn the_folder_a_path_lands_in_is_worked_out_without_touching_the_disk() {
    let (root, dest) = setup();
    assert_eq!(dest.folder_of(&path("a.txt")), "");
    assert_eq!(dest.folder_of(&path("Docs/Tax/a.txt")), "Docs/Tax");
    if cfg!(windows) {
        assert_eq!(dest.folder_of(&path("CON/a.txt")), "CON_");
    }
    assert!(names(root.path()).is_empty());
}

#[cfg(unix)]
#[test]
fn a_link_in_place_of_a_temporary_file_is_never_followed() {
    let (root, dest) = setup();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("secret"), b"s").unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("secret"),
        root.path().join(".pctwin-t-1.part"),
    )
    .unwrap();
    assert!(
        dest.reopen_sealed(&path("a.txt"), ".pctwin-t-1.part")
            .is_err()
    );
    assert_eq!(dest.look(".pctwin-t-1.part").unwrap(), None);
    assert!(!dest.remove_temp(".pctwin-t-1.part").unwrap());
    assert!(outside.path().join("secret").exists());
}

#[test]
fn a_partly_received_file_is_reopened_and_only_checked_bytes_count_as_arrived() {
    let (root, dest) = setup();
    std::fs::create_dir(root.path().join("d")).unwrap();
    std::fs::write(root.path().join("d/.pctwin-t-1.part"), b"abcdef").unwrap();
    let mut file = dest
        .reopen_incoming(&path("d/a.txt"), "d/.pctwin-t-1.part", 10)
        .unwrap();
    assert_eq!(file.temp_path(), "d/.pctwin-t-1.part");
    assert!(file.created_folders().is_empty());
    // Nothing counts until checked.
    assert_eq!(file.written(), 0);
    let mut back = [0u8; 3];
    file.read_at(2, &mut back).unwrap();
    assert_eq!(&back, b"cde");
    file.count_arrived(0, 6).unwrap();
    assert_eq!(file.written(), 6);
    // Counted bytes are never counted or written twice; nothing outside the file.
    assert!(matches!(file.count_arrived(5, 1), Err(GateError::Overlap)));
    assert!(matches!(file.write_at(4, b"x"), Err(GateError::Overlap)));
    assert!(matches!(
        file.count_arrived(8, 3),
        Err(GateError::OutsideFile)
    ));
    file.write_at(6, b"ghij").unwrap();
    let done = file.finish().unwrap();
    assert_eq!(done.final_path, "d/a.txt");
    assert_eq!(
        std::fs::read(root.path().join("d/a.txt")).unwrap(),
        b"abcdefghij"
    );
}

#[test]
fn reopening_refuses_anything_but_a_pctwin_temporary_file_no_longer_than_announced() {
    let (root, dest) = setup();
    std::fs::write(root.path().join("a.txt"), b"mine").unwrap();
    std::fs::write(root.path().join(".pctwin-t-1.part"), b"0123456789").unwrap();
    std::fs::create_dir(root.path().join(".pctwin-t-2.part")).unwrap();
    for (stored, announced) in [
        ("a.txt", 10),
        (".pctwin-t-1.part", 9),
        (".pctwin-t-2.part", 10),
        (".pctwin-t-3.part", 10),
        ("../.pctwin-t-1.part", 10),
    ] {
        assert!(
            dest.reopen_incoming(&path("b.txt"), stored, announced)
                .is_err(),
            "{stored:?}"
        );
    }
    assert_eq!(std::fs::read(root.path().join("a.txt")).unwrap(), b"mine");
    assert_eq!(
        std::fs::read(root.path().join(".pctwin-t-1.part")).unwrap(),
        b"0123456789"
    );
}

#[test]
fn a_reopened_file_dropped_unfinished_is_removed() {
    let (root, dest) = setup();
    std::fs::write(root.path().join(".pctwin-t-1.part"), b"abc").unwrap();
    let file = dest
        .reopen_incoming(&path("a.txt"), ".pctwin-t-1.part", 5)
        .unwrap();
    drop(file);
    assert!(names(root.path()).is_empty());
}

#[test]
fn only_an_empty_folder_is_ever_removed() {
    let (root, dest) = setup();
    std::fs::create_dir_all(root.path().join("a/empty")).unwrap();
    std::fs::create_dir_all(root.path().join("a/full")).unwrap();
    std::fs::write(root.path().join("a/full/mine.txt"), b"mine").unwrap();
    std::fs::write(root.path().join("a/file"), b"x").unwrap();
    assert!(dest.remove_empty_folder("a/empty").unwrap());
    assert!(!root.path().join("a/empty").exists());
    assert!(!dest.remove_empty_folder("a/full").unwrap());
    assert_eq!(
        std::fs::read(root.path().join("a/full/mine.txt")).unwrap(),
        b"mine"
    );
    assert!(!dest.remove_empty_folder("a/file").unwrap());
    assert!(root.path().join("a/file").exists());
    assert!(!dest.remove_empty_folder("a/gone").unwrap());
    assert!(dest.remove_empty_folder("../a").is_err());
}

#[test]
fn a_path_for_the_trash_is_given_only_for_exactly_the_file_expected() {
    let (root, dest) = setup();
    std::fs::create_dir(root.path().join("d")).unwrap();
    std::fs::write(root.path().join("d/a.txt"), b"a").unwrap();
    std::fs::write(root.path().join("d/b.txt"), b"a").unwrap();
    let a = dest.stat("d/a.txt").unwrap().unwrap().id;
    let path = dest.ambient_path("d/a.txt", a).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"a");
    assert_eq!(
        path,
        std::fs::canonicalize(root.path().join("d/a.txt")).unwrap()
    );
    // Another file, even with the same contents, is never given.
    assert!(dest.ambient_path("d/b.txt", a).is_err());
    assert!(dest.ambient_path("d/x.txt", a).is_err());
    assert!(dest.ambient_path("d", a).is_err());
    assert!(dest.ambient_path("../d/a.txt", a).is_err());
    // A place opened through the admin helper has no path to give.
    let mut table = pctwin_gate::Destinations::new();
    let dir =
        pctwin_gate::Dir::open_ambient_dir(root.path(), cap_std::ambient_authority()).unwrap();
    table.approve_through_helper("them", "1002", dir).unwrap();
    assert!(
        table
            .get("them")
            .unwrap()
            .ambient_path("d/a.txt", a)
            .is_err()
    );
}

#[cfg(unix)]
#[test]
fn a_path_through_a_link_is_never_given_for_the_trash() {
    let (root, dest) = setup();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("a.txt"), b"theirs").unwrap();
    std::os::unix::fs::symlink(outside.path(), root.path().join("d")).unwrap();
    let file = std::fs::File::open(outside.path().join("a.txt")).unwrap();
    use std::os::unix::fs::MetadataExt;
    let meta = file.metadata().unwrap();
    let id = pctwin_gate::FileId {
        volume: meta.dev(),
        index: meta.ino(),
    };
    assert!(dest.ambient_path("d/a.txt", id).is_err());
}

#[cfg(windows)]
#[test]
fn a_path_through_a_junction_is_never_given_for_the_trash() {
    let (root, dest) = setup();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("a.txt"), b"theirs").unwrap();
    // A junction needs no special rights on Windows.
    let made = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(root.path().join("d"))
        .arg(outside.path())
        .output()
        .unwrap();
    assert!(made.status.success(), "{made:?}");
    let theirs = Destination::open(outside.path()).unwrap();
    let id = theirs.stat("a.txt").unwrap().unwrap().id;
    assert!(dest.ambient_path("d/a.txt", id).is_err());
    assert_eq!(
        std::fs::read(outside.path().join("a.txt")).unwrap(),
        b"theirs"
    );
}
