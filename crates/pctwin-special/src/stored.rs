//! One value in an app's key-value database, read and replaced in PCTwin's parser process
//! (Task List 2.4: recent-file paths rewritten).
//!
//! Some apps keep a list of recent files as one row of a key-value table in a SQLite database
//! (VS Code's `history.recentlyOpenedPathsList` in `state.vscdb`'s `ItemTable`). The settings
//! catalogue names the file, the table and the key; nothing else in the database is read or
//! changed. Both functions take an [`InChild`]: they run only in the parser process, on a
//! bounded copy of the database's bytes, never on a file.
//!
//! Replacing a value works on an in-memory copy: triggers are switched off (a trigger could
//! change other rows), exactly one row must change, SQLite's own `quick_check` must pass, and the
//! header's journal bytes are put back as they were. The rest of the database keeps its meaning;
//! the bytes handed back are a complete database.

use pctwin_parse_seal::InChild;
use rusqlite::config::DbConfig;
use rusqlite::types::Value as SqlValue;
use serde::{Deserialize, Serialize};

use crate::content::{ContentError, MAX_DB, image_of};

/// The largest value read or written.
pub const MAX_STORED: usize = 1024 * 1024;

/// Which row: a key in a table with `key` and `value` columns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredKey {
    pub table: String,
    pub key: String,
}

/// A row's new value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredChange {
    pub table: String,
    pub key: String,
    pub value: String,
}

/// A table name as the catalogue may give it: letters, digits and `_`, not starting with a
/// digit, at most 64 long.
pub fn table_name_ok(t: &str) -> bool {
    !t.is_empty()
        && t.len() <= 64
        && !t.starts_with(|c: char| c.is_ascii_digit())
        && t.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn damaged(e: impl std::fmt::Display) -> ContentError {
    ContentError::Damaged(e.to_string())
}

fn open(image: &[u8], read_only: bool) -> Result<rusqlite::Connection, ContentError> {
    let mut c = rusqlite::Connection::open_in_memory().map_err(damaged)?;
    for (flag, on) in [
        (DbConfig::SQLITE_DBCONFIG_ENABLE_TRIGGER, false),
        (DbConfig::SQLITE_DBCONFIG_ENABLE_VIEW, false),
        (DbConfig::SQLITE_DBCONFIG_TRUSTED_SCHEMA, false),
        (DbConfig::SQLITE_DBCONFIG_DEFENSIVE, true),
    ] {
        c.set_db_config(flag, on).map_err(damaged)?;
    }
    if !image.is_empty() {
        c.deserialize_read_exact(rusqlite::MAIN_DB, image, image.len(), read_only)
            .map_err(damaged)?;
    }
    Ok(c)
}

/// Whether `table` is a real table (not a view) with `key` and `value` columns.
fn check_table(c: &rusqlite::Connection, table: &str) -> Result<(), ContentError> {
    if !table_name_ok(table) {
        return Err(damaged("not a table name"));
    }
    let kind: Option<String> = c
        .query_row(
            "SELECT type FROM sqlite_schema WHERE name = ?1 COLLATE NOCASE",
            [table],
            |r| r.get(0),
        )
        .ok();
    if kind.as_deref() != Some("table") {
        return Err(damaged("it has no such table"));
    }
    let mut cols = c
        .prepare(&format!("SELECT name FROM pragma_table_info('{table}')"))
        .map_err(damaged)?;
    let names: Vec<String> = cols
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(damaged)?
        .flatten()
        .map(|n| n.to_ascii_lowercase())
        .collect();
    if !(names.iter().any(|n| n == "key") && names.iter().any(|n| n == "value")) {
        return Err(damaged("its table has no key and value"));
    }
    Ok(())
}

/// The rows under `key` (text or UTF-8 bytes), with whether it was stored as bytes.
fn rows(
    c: &rusqlite::Connection,
    table: &str,
    key: &str,
) -> Result<Vec<(String, bool)>, ContentError> {
    let mut q = c
        .prepare(&format!("SELECT value FROM \"{table}\" WHERE key = ?1"))
        .map_err(damaged)?;
    let mut out = Vec::new();
    let mut it = q.query([key]).map_err(damaged)?;
    while let Some(r) = it.next().map_err(damaged)? {
        let v: SqlValue = r.get(0).map_err(damaged)?;
        let (text, blob) = match v {
            SqlValue::Text(t) => (t, false),
            SqlValue::Blob(b) => (String::from_utf8(b).map_err(damaged)?, true),
            _ => return Err(damaged("its value is not text")),
        };
        if text.len() > MAX_STORED {
            return Err(ContentError::TooLarge);
        }
        out.push((text, blob));
        if out.len() > 1 {
            return Err(damaged("the key is there more than once"));
        }
    }
    Ok(out)
}

/// The value under `want.key` in `want.table`, or `None` when the key is not there. In the
/// parser process (it takes an [`InChild`]).
pub fn stored_value(
    seal: InChild,
    db: &[u8],
    want: &StoredKey,
) -> Result<Option<String>, ContentError> {
    let image = image_of(seal, db, None)?;
    let c = open(&image, true)?;
    check_table(&c, &want.table)?;
    Ok(rows(&c, &want.table, &want.key)?.pop().map(|(t, _)| t))
}

/// The database with the value under `change.key` replaced (stored the way it was: text or
/// bytes), everything else as it was. The key must be there exactly once. In the parser process
/// (it takes an [`InChild`]).
pub fn replace_value(
    seal: InChild,
    db: &[u8],
    change: &StoredChange,
) -> Result<Vec<u8>, ContentError> {
    if change.value.len() > MAX_STORED {
        return Err(ContentError::TooLarge);
    }
    if db.len() < 100 {
        return Err(damaged("file is not a database"));
    }
    let journal = (db[18], db[19]);
    let image = image_of(seal, db, None)?;
    let c = open(&image, false)?;
    check_table(&c, &change.table)?;
    let was = rows(&c, &change.table, &change.key)?;
    let Some((_, blob)) = was.first() else {
        return Err(damaged("it has no such key"));
    };
    let value = if *blob {
        SqlValue::Blob(change.value.clone().into_bytes())
    } else {
        SqlValue::Text(change.value.clone())
    };
    let n = c
        .execute(
            &format!("UPDATE \"{}\" SET value = ?1 WHERE key = ?2", change.table),
            rusqlite::params![value, change.key],
        )
        .map_err(damaged)?;
    if n != 1 {
        return Err(damaged("more than the one row would change"));
    }
    let verdict: String = c
        .query_row("PRAGMA quick_check", [], |r| r.get(0))
        .map_err(damaged)?;
    if verdict != "ok" {
        return Err(damaged(verdict));
    }
    let mut c = c;
    let mut out = c.serialize(rusqlite::MAIN_DB).map_err(damaged)?.to_vec();
    if out.len() as u64 > MAX_DB || out.len() < 100 {
        return Err(ContentError::TooLarge);
    }
    (out[18], out[19]) = journal;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEAL: InChild = pctwin_parse_seal::in_child();

    /// A database as VS Code leaves `state.vscdb`, with a second table and a trigger.
    fn vscdb(dir: &std::path::Path, recent: &str) -> Vec<u8> {
        let p = dir.join("state.vscdb");
        let c = rusqlite::Connection::open(&p).unwrap();
        c.execute_batch(
            "CREATE TABLE ItemTable (key TEXT UNIQUE ON CONFLICT REPLACE, value BLOB);
             CREATE TABLE other (x);
             INSERT INTO other VALUES ('untouched');
             CREATE TRIGGER t AFTER UPDATE ON ItemTable BEGIN DELETE FROM other; END;",
        )
        .unwrap();
        c.execute(
            "INSERT INTO ItemTable VALUES ('history.recentlyOpenedPathsList', ?1)",
            [recent],
        )
        .unwrap();
        c.execute("INSERT INTO ItemTable VALUES ('theme', X'6461726b')", [])
            .unwrap();
        drop(c);
        std::fs::read(&p).unwrap()
    }

    fn key() -> StoredKey {
        StoredKey {
            table: "ItemTable".into(),
            key: "history.recentlyOpenedPathsList".into(),
        }
    }

    #[test]
    fn one_value_is_read_and_replaced_and_nothing_else_changes() {
        let dir = tempfile::tempdir().unwrap();
        let db = vscdb(dir.path(), r#"{"entries":[]}"#);
        assert_eq!(
            stored_value(SEAL, &db, &key()).unwrap().as_deref(),
            Some(r#"{"entries":[]}"#)
        );
        let new = replace_value(
            SEAL,
            &db,
            &StoredChange {
                table: "ItemTable".into(),
                key: key().key,
                value: r#"{"entries":[1]}"#.into(),
            },
        )
        .unwrap();
        assert_eq!((new[18], new[19]), (db[18], db[19]));
        assert_eq!(
            stored_value(SEAL, &new, &key()).unwrap().as_deref(),
            Some(r#"{"entries":[1]}"#)
        );
        // The trigger did not run; the other rows and table are as they were.
        let p = dir.path().join("after.vscdb");
        std::fs::write(&p, &new).unwrap();
        let c = rusqlite::Connection::open(&p).unwrap();
        let other: String = c
            .query_row("SELECT x FROM other", [], |r| r.get(0))
            .unwrap();
        assert_eq!(other, "untouched");
        let theme: Vec<u8> = c
            .query_row("SELECT value FROM ItemTable WHERE key='theme'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(theme, b"dark");
    }

    #[test]
    fn a_missing_key_a_bad_table_and_not_a_database_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let db = vscdb(dir.path(), "[]");
        let missing = StoredKey {
            table: "ItemTable".into(),
            key: "nope".into(),
        };
        assert_eq!(stored_value(SEAL, &db, &missing).unwrap(), None);
        let change = |table: &str, key: &str| StoredChange {
            table: table.into(),
            key: key.into(),
            value: "x".into(),
        };
        assert!(replace_value(SEAL, &db, &change("ItemTable", "nope")).is_err());
        assert!(replace_value(SEAL, &db, &change("Item\"Table", "theme")).is_err());
        assert!(replace_value(SEAL, &db, &change("other", "theme")).is_err());
        assert!(replace_value(SEAL, b"not a database at all", &change("ItemTable", "x")).is_err());
        assert!(!table_name_ok("1abc"));
        assert!(!table_name_ok("a b"));
        assert!(table_name_ok("ItemTable"));
    }
}
