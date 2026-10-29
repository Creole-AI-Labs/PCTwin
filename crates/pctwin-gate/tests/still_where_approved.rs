//! Task List 2.9: a destination folder moved or replaced after it was approved is told apart from
//! the folder held, so a move never writes into a place the person did not see. On Windows the
//! held folder itself cannot be moved (its handle forbids it), but the folder above it can be.

use pctwin_gate::{Approved, Destination, Destinations};

/// Moves `from` to `to`, then puts a new empty folder at `from`; `false` if the system refused
/// the move (the folder was not changed).
fn swap(from: &std::path::Path, to: &std::path::Path) -> bool {
    if std::fs::rename(from, to).is_err() {
        return false;
    }
    std::fs::create_dir_all(from).unwrap();
    true
}

#[test]
fn a_folder_moved_or_replaced_after_approval_is_no_longer_where_it_was_approved() {
    let t = tempfile::tempdir().unwrap();
    let docs = t.path().join("Documents");
    std::fs::create_dir(&docs).unwrap();
    let mut table = Destinations::new();
    table
        .approve("documents", Approved::MyFolders, &docs)
        .unwrap();
    assert!(table.get("documents").unwrap().still_where_approved());
    assert!(table.changed_since_approved().is_empty());
    // The approved folder itself moved and another put at its path (refused on Windows while
    // PCTwin holds it, so nothing changed there).
    let moved = swap(&docs, &t.path().join("Documents old"));
    assert_eq!(cfg!(windows), !moved);
    assert_eq!(table.changed_since_approved().is_empty(), !moved);
}

#[test]
fn the_folder_above_moved_after_approval_is_seen_too() {
    let t = tempfile::tempdir().unwrap();
    let above = t.path().join("Stuff");
    let docs = above.join("Documents");
    std::fs::create_dir_all(&docs).unwrap();
    let mut table = Destinations::new();
    table
        .approve("documents", Approved::MyFolders, &docs)
        .unwrap();
    let moved = swap(&above, &t.path().join("Stuff old"));
    if moved {
        std::fs::create_dir_all(&docs).unwrap();
        assert_eq!(table.changed_since_approved(), ["documents"]);
    } else {
        assert!(table.changed_since_approved().is_empty());
    }
    // A folder opened just now is where it was opened.
    assert!(Destination::open(&docs).unwrap().still_where_approved());
}
