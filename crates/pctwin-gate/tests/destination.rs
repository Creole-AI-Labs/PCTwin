//! Writing on the new laptop: only inside the approved folder, through a handle (cap-std), never
//! overwriting, and never more or fewer bytes than the old laptop announced.

use std::io::Write;

use pctwin_gate::{Destination, GateError, IncomingPath, NameChange};

fn path(s: &str) -> IncomingPath {
    IncomingPath::parse(s).unwrap()
}

fn write(dest: &Destination, p: &str, bytes: &[u8]) -> Result<String, GateError> {
    let mut file = dest.create_file(&path(p), bytes.len() as u64)?;
    file.write_all(bytes).map_err(GateError::Io)?;
    let done = file.finish()?;
    Ok(done.final_path)
}

#[test]
fn a_file_is_written_inside_the_approved_folder_with_its_folders() {
    let root = tempfile::tempdir().unwrap();
    let dest = Destination::open(root.path()).unwrap();
    let written = write(&dest, "Documents/Taxes/return.pdf", b"PDF").unwrap();
    assert_eq!(written, "Documents/Taxes/return.pdf");
    assert_eq!(
        std::fs::read(root.path().join("Documents/Taxes/return.pdf")).unwrap(),
        b"PDF"
    );
}

#[test]
fn an_existing_file_is_never_overwritten() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("notes.txt"), b"mine").unwrap();
    let dest = Destination::open(root.path()).unwrap();
    let mut file = dest.create_file(&path("notes.txt"), 6).unwrap();
    file.write_all(b"theirs").unwrap();
    let done = file.finish().unwrap();
    assert_eq!(done.final_path, "notes (2).txt");
    assert!(done.changes.contains(&NameChange::NameClash));
    assert_eq!(
        std::fs::read(root.path().join("notes.txt")).unwrap(),
        b"mine"
    );
    assert_eq!(
        std::fs::read(root.path().join("notes (2).txt")).unwrap(),
        b"theirs"
    );
}

#[test]
fn names_that_clash_only_by_case_are_both_kept_where_the_disk_ignores_case() {
    let root = tempfile::tempdir().unwrap();
    let dest = Destination::open(root.path()).unwrap();
    let first = write(&dest, "Photo.jpg", b"1").unwrap();
    let second = write(&dest, "photo.jpg", b"2").unwrap();
    assert_eq!(first, "Photo.jpg");
    // On a case-insensitive disk (Windows, Mac) the second gets a new name; on Linux it fits.
    assert!(
        second == "photo.jpg" || second == "photo (2).jpg",
        "{second}"
    );
    let mut contents: Vec<Vec<u8>> = std::fs::read_dir(root.path())
        .unwrap()
        .map(|e| std::fs::read(e.unwrap().path()).unwrap())
        .collect();
    contents.sort();
    assert_eq!(contents, [b"1".to_vec(), b"2".to_vec()], "both files kept");
}

#[test]
fn names_this_system_cannot_store_are_changed_and_reported() {
    let root = tempfile::tempdir().unwrap();
    let dest = Destination::open(root.path()).unwrap();
    let mut file = dest.create_file(&path("CON.txt"), 1).unwrap();
    file.write_all(b"x").unwrap();
    let done = file.finish().unwrap();
    if cfg!(windows) {
        assert_eq!(done.final_path, "CON_.txt");
        assert!(done.changes.contains(&NameChange::ReservedName));
    } else {
        assert_eq!(done.final_path, "CON.txt");
    }
}

#[cfg(windows)]
#[test]
fn a_colon_never_creates_a_hidden_data_stream_on_windows() {
    let root = tempfile::tempdir().unwrap();
    let dest = Destination::open(root.path()).unwrap();
    let written = write(&dest, "report.txt:hidden", b"secret").unwrap();
    assert_eq!(written, "report.txt\u{FF1A}hidden");
    assert!(
        !root.path().join("report.txt").exists(),
        "no host file was created for a stream"
    );
}

#[test]
fn more_bytes_than_announced_are_refused() {
    let root = tempfile::tempdir().unwrap();
    let dest = Destination::open(root.path()).unwrap();
    let mut file = dest.create_file(&path("a.bin"), 4).unwrap();
    file.write_all(b"1234").unwrap();
    let err = file.write_all(b"5").unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
}

#[test]
fn fewer_bytes_than_announced_is_not_a_finished_file() {
    let root = tempfile::tempdir().unwrap();
    let dest = Destination::open(root.path()).unwrap();
    let mut file = dest.create_file(&path("a.bin"), 4).unwrap();
    file.write_all(b"12").unwrap();
    assert!(matches!(
        file.finish(),
        Err(GateError::SizeMismatch {
            announced: 4,
            received: 2
        })
    ));
}

#[test]
fn a_folder_cannot_be_written_where_a_file_already_is() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("Documents"), b"a file").unwrap();
    let dest = Destination::open(root.path()).unwrap();
    assert!(matches!(
        dest.create_file(&path("Documents/a.txt"), 1),
        Err(GateError::Conflict)
    ));
    assert_eq!(
        std::fs::read(root.path().join("Documents")).unwrap(),
        b"a file"
    );
}

#[cfg(unix)]
#[test]
fn a_link_inside_the_folder_cannot_redirect_a_write_outside_it() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(outside.path(), root.path().join("escape")).unwrap();
    let dest = Destination::open(root.path()).unwrap();
    assert!(dest.create_file(&path("escape/stolen.txt"), 1).is_err());
    assert!(!outside.path().join("stolen.txt").exists());
}

#[cfg(unix)]
#[test]
fn a_link_named_like_the_file_is_not_followed() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let target = outside.path().join("victim.txt");
    std::fs::write(&target, b"keep").unwrap();
    std::os::unix::fs::symlink(&target, root.path().join("note.txt")).unwrap();
    let dest = Destination::open(root.path()).unwrap();
    let written = write(&dest, "note.txt", b"new").unwrap();
    assert_ne!(written, "note.txt", "the link's name is treated as taken");
    assert_eq!(std::fs::read(&target).unwrap(), b"keep");
}
