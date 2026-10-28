//! One undo book per move (gap 4 of the complications audit, 28 October 2026).
//!
//! A full move's undo book says which settings, apps and app data its move changed, by that
//! move's own scope in the journal. Each book is kept in its own file, named by its move's ID, in
//! one folder beside the journal: a second move never writes over the first one's book, so each
//! move can still be undone on its own, as the journal already keeps every move's files apart.
//!
//! PCTwin 0.1.x kept one book (`fullmove-undo.json`) for the last move only. That file is still
//! read: it is copied into the folder under its own move's ID, read back, and only then removed;
//! a file that cannot be read is left exactly where it is, never dropped.

use std::path::{Path, PathBuf};

use pctwin_fullmove::run::UndoBook;

/// The folder of books, beside the journal.
pub const FOLDER: &str = "fullmove-undo";
/// The one book PCTwin 0.1.x kept, beside the journal.
pub const LEGACY: &str = "fullmove-undo.json";
/// The largest book read (a book is a few kilobytes; anything far larger is not one).
const MAX_BOOK: u64 = 16 * 1024 * 1024;

fn is_id(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

fn path_of(folder: &Path, id: &str) -> Option<PathBuf> {
    is_id(id).then(|| folder.join(format!("{id}.json")))
}

fn read(path: &Path) -> Option<UndoBook> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > MAX_BOOK {
        return None;
    }
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

/// Keeps `book` in `folder` under its move's ID: written to a temporary name, flushed, renamed
/// into place, then read back; an error unless what is there now is exactly this book.
pub fn save(folder: &Path, book: &UndoBook) -> std::io::Result<()> {
    use std::io::Write;
    let id = book.id();
    let path = path_of(folder, &id).ok_or_else(|| std::io::Error::other("not a move ID"))?;
    std::fs::create_dir_all(folder)?;
    let bytes = serde_json::to_vec(book).map_err(std::io::Error::other)?;
    let temp = folder.join(format!(".{id}.part"));
    {
        let mut f = std::fs::File::create(&temp)?;
        f.write_all(&bytes)?;
        f.sync_all()?;
    }
    std::fs::rename(&temp, &path)?;
    if read(&path).as_ref() == Some(book) {
        Ok(())
    } else {
        Err(std::io::Error::other("the undo book did not read back"))
    }
}

/// Copies PCTwin 0.1.x's one book (`legacy`) into `folder`, if it is there and readable and its
/// move has no book yet; removes it only once the copy reads back. Never drops a book.
pub fn migrate(folder: &Path, legacy: &Path) {
    let Some(book) = read(legacy) else {
        return;
    };
    let Some(path) = path_of(folder, &book.id()) else {
        return;
    };
    let kept = match read(&path) {
        Some(there) => there == book,
        None => save(folder, &book).is_ok(),
    };
    if kept {
        let _ = std::fs::remove_file(legacy);
    }
}

/// Every move's book in `folder`, newest first (a 0.1.x book, with no time, last).
pub fn all(folder: &Path) -> Vec<UndoBook> {
    let mut books: Vec<UndoBook> = std::fs::read_dir(folder)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let id = name.strip_suffix(".json")?;
            // Only a book whose name is its own move's ID.
            read(&path_of(folder, id)?).filter(|b| b.id() == id)
        })
        .collect();
    books.sort_by(|a, b| b.when.cmp(&a.when).then_with(|| a.id().cmp(&b.id())));
    books
}

/// The book of the move `id`, or (`None`) the newest move's.
pub fn find(folder: &Path, id: Option<&str>) -> Option<UndoBook> {
    match id {
        Some(id) => read(&path_of(folder, id)?).filter(|b| b.id() == id),
        None => all(folder).into_iter().next(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn book(n: u8, when: Option<u64>) -> UndoBook {
        UndoBook {
            scope: [n; 16],
            apps: vec![format!("App {n}")],
            when,
            ..UndoBook::default()
        }
    }

    /// A second move keeps its own book: the first move's is still there, whole, and each is
    /// found by its own move's ID; with no ID the newest move's is the one undone.
    #[test]
    fn a_second_move_never_erases_the_first_moves_undo_book() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join(FOLDER);
        let (first, second) = (book(1, Some(100)), book(2, Some(200)));
        save(&folder, &first).unwrap();
        save(&folder, &second).unwrap();
        assert_eq!(find(&folder, Some(&first.id())), Some(first.clone()));
        assert_eq!(find(&folder, Some(&second.id())), Some(second.clone()));
        assert_eq!(find(&folder, None), Some(second.clone()));
        assert_eq!(all(&folder), [second, first]);
        // Not a move's ID: nothing, never a path outside the folder.
        assert_eq!(find(&folder, Some("../fullmove-undo")), None);
    }

    /// The one book PCTwin 0.1.x kept is still read: moved into the folder under its move's ID
    /// (read back first), then removed. One that cannot be read is left exactly where it is.
    #[test]
    fn the_old_single_book_is_migrated_never_dropped() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join(FOLDER);
        let legacy = dir.path().join(LEGACY);
        // As 0.1.x wrote it: no `when` field.
        let old = book(7, None);
        let mut json: serde_json::Value = serde_json::to_value(&old).unwrap();
        json.as_object_mut().unwrap().remove("when");
        std::fs::write(&legacy, serde_json::to_vec(&json).unwrap()).unwrap();
        save(&folder, &book(8, Some(5))).unwrap();
        migrate(&folder, &legacy);
        assert!(!legacy.exists());
        assert_eq!(find(&folder, Some(&old.id())), Some(old.clone()));
        assert_eq!(all(&folder).len(), 2);
        // A damaged one stays as it is.
        std::fs::write(&legacy, b"{ not a book").unwrap();
        migrate(&folder, &legacy);
        assert_eq!(std::fs::read(&legacy).unwrap(), b"{ not a book");
    }
}
