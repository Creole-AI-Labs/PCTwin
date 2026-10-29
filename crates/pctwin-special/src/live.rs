//! Files in the person's own folders that are live while their app runs (Task List 2.5, decided
//! 29 October 2026): a mail store such as an Outlook data file, a database, a virtual disk. A copy
//! made while the app writes can be torn, so such a file is not copied while its app holds it: the
//! old laptop asks the person to close only the app holding it, and a file left open ends
//! needs-action, never shown copied.
//!
//! Which files are such a store is data, not code: the built-in catalogue (`data/live_files.json`)
//! lists kinds, each by the endings its files have and, where the bytes can show it, the shapes
//! ([`crate::sniff`]) they must have (a `.db` file that is not a database, like a picture cache, is
//! not one). Which app holds a file is never in the catalogue: the system names it at the time.

use std::sync::OnceLock;

use serde::Deserialize;

use crate::sniff::Shape;

/// A shape a kind's files may have, by the name the catalogue uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ShapeName {
    OutlookData,
    Sqlite,
    LmdbData,
    LeveldbTable,
    /// A Microsoft Access or Jet database (its header names the engine).
    OfficeDatabase,
}

impl ShapeName {
    fn fits(self, shape: &Shape, head: &[u8]) -> bool {
        match self {
            ShapeName::OutlookData => matches!(shape, Shape::OutlookData { .. }),
            ShapeName::Sqlite => *shape == Shape::Sqlite,
            ShapeName::LmdbData => *shape == Shape::LmdbData,
            ShapeName::LeveldbTable => *shape == Shape::LevelDbTable,
            ShapeName::OfficeDatabase => matches!(
                head.get(4..19),
                Some(b"Standard Jet DB" | b"Standard ACE DB")
            ),
        }
    }
}

/// One kind of live file.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Kind {
    /// The kind, as the screens may name it (`mail-store`, `database`, ...).
    pub kind: String,
    /// The endings its files have, lower case, without the dot.
    pub endings: Vec<String>,
    /// The shapes its bytes must show, one of them; empty: the ending is enough.
    pub shapes: Vec<ShapeName>,
    /// Files its app keeps beside it that belong with it, by what is added to its whole name
    /// (an SQLite database's `-wal`, `-shm` and `-journal`): held and read together with it.
    #[serde(default)]
    pub sidecars: Vec<String>,
}

/// The catalogue of live kinds.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiveKinds {
    #[serde(default)]
    pub about: String,
    pub kinds: Vec<Kind>,
}

/// Why a catalogue was refused.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum LiveKindsError {
    #[error("the catalogue is not readable: {0}")]
    Unreadable(String),
    #[error(
        "a kind has no name or no endings, or an ending that is not plain lower-case letters and digits"
    )]
    BadKind,
}

fn plain_ending(e: &str) -> bool {
    !e.is_empty()
        && e.len() <= 16
        && e.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
}

impl LiveKinds {
    /// Reads a catalogue, refusing one with an empty kind or a doubtful ending.
    pub fn parse(json: &str) -> Result<Self, LiveKindsError> {
        let k: LiveKinds =
            serde_json::from_str(json).map_err(|e| LiveKindsError::Unreadable(e.to_string()))?;
        let plain_sidecar = |s: &String| s.strip_prefix('-').is_some_and(plain_ending);
        if k.kinds.iter().any(|kind| {
            kind.kind.is_empty()
                || kind.endings.is_empty()
                || !kind.endings.iter().all(|e| plain_ending(e))
                || !kind.sidecars.iter().all(plain_sidecar)
        }) {
            return Err(LiveKindsError::BadKind);
        }
        Ok(k)
    }

    /// The built-in catalogue.
    pub fn built_in() -> &'static LiveKinds {
        static BUILT_IN: OnceLock<LiveKinds> = OnceLock::new();
        BUILT_IN.get_or_init(|| {
            // Checked by this crate's tests; a broken built-in catalogue knows no kinds rather
            // than stopping the move.
            LiveKinds::parse(include_str!("../data/live_files.json")).unwrap_or(LiveKinds {
                about: String::new(),
                kinds: Vec::new(),
            })
        })
    }

    /// Whether a file named `name` may be one of the kinds (its ending is a kind's): only those
    /// are looked at more closely.
    pub fn may_be(&self, name: &str) -> bool {
        !self.kinds_ending(name).is_empty()
    }

    fn kinds_ending(&self, name: &str) -> Vec<&Kind> {
        let Some((stem, ending)) = name.rsplit_once('.') else {
            return Vec::new();
        };
        if stem.is_empty() {
            return Vec::new();
        }
        let ending = ending.to_ascii_lowercase();
        self.kinds
            .iter()
            .filter(|k| k.endings.contains(&ending))
            .collect()
    }

    /// The kind of a file named `name` of `len` bytes whose first bytes are `head` (up to
    /// [`crate::sniff::HEAD`]) and last are `tail`, or `None` when it is not a live file.
    pub fn kind_of(&self, name: &str, head: &[u8], tail: &[u8], len: u64) -> Option<&Kind> {
        let kinds = self.kinds_ending(name);
        if kinds.is_empty() {
            return None;
        }
        let shape = crate::sniff::shape(head, tail, len);
        kinds
            .into_iter()
            .find(|k| k.shapes.is_empty() || k.shapes.iter().any(|s| s.fits(&shape, head)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sqlite_head() -> Vec<u8> {
        let mut h = b"SQLite format 3\0".to_vec();
        h.resize(100, 0);
        h[16] = 0x10;
        h[18] = 1;
        h[19] = 1;
        h[21] = 64;
        h[22] = 32;
        h[23] = 32;
        h
    }

    #[test]
    fn the_built_in_catalogue_reads_and_knows_mail_stores_and_databases() {
        let k = LiveKinds::parse(include_str!("../data/live_files.json")).unwrap();
        assert!(!k.kinds.is_empty());
        let b = LiveKinds::built_in();
        assert_eq!(*b, k);
        assert!(b.may_be("Mail.PST"));
        assert!(b.may_be("books.sqlite"));
        assert!(!b.may_be("letter.docx"));
        assert!(!b.may_be("pst"));
        assert!(!b.may_be(".db"));
        let db = b
            .kinds
            .iter()
            .find(|k| k.endings.contains(&"sqlite".to_string()))
            .unwrap();
        assert_eq!(db.sidecars, ["-wal", "-shm", "-journal"]);
    }

    #[test]
    fn a_database_ending_counts_only_when_its_bytes_are_a_database() {
        let k = LiveKinds::built_in();
        let h = sqlite_head();
        assert_eq!(
            k.kind_of("money.db", &h, &[], 4096)
                .map(|k| k.kind.as_str()),
            Some("database")
        );
        // A picture cache with the same ending is not a database.
        assert_eq!(
            k.kind_of("Thumbs.db", b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1", &[], 4096),
            None
        );
        // A database with another ending is not looked at here.
        assert_eq!(k.kind_of("money.txt", &h, &[], 4096), None);
    }

    #[test]
    fn a_mail_store_is_told_by_its_bytes() {
        let k = LiveKinds::built_in();
        let mut h = b"!BDN".to_vec();
        h.extend_from_slice(&[0, 0, 0, 0]);
        h.extend_from_slice(b"SM");
        h.resize(512, 0);
        assert_eq!(
            k.kind_of("Outlook.pst", &h, &[], 4096)
                .map(|k| k.kind.as_str()),
            Some("mail-store")
        );
        assert_eq!(k.kind_of("Outlook.pst", b"not mail", &[], 8), None);
    }

    #[test]
    fn a_kind_without_shapes_goes_by_its_ending() {
        let k = LiveKinds::built_in();
        assert_eq!(
            k.kind_of("disk.VHDX", b"anything", &[], 8)
                .map(|k| k.kind.as_str()),
            Some("virtual-disk")
        );
    }

    #[test]
    fn an_office_database_is_told_by_its_engine_name() {
        let k = LiveKinds::built_in();
        let mut h = vec![0, 1, 0, 0];
        h.extend_from_slice(b"Standard ACE DB\0");
        assert_eq!(
            k.kind_of("shop.accdb", &h, &[], 4096)
                .map(|k| k.kind.as_str()),
            Some("office-database")
        );
        assert_eq!(
            k.kind_of("shop.accdb", b"\0\0\0\0Something else", &[], 4096),
            None
        );
    }

    #[test]
    fn a_doubtful_catalogue_is_refused() {
        for bad in [
            r#"{"kinds":[{"kind":"","endings":["db"],"shapes":[]}]}"#,
            r#"{"kinds":[{"kind":"x","endings":[],"shapes":[]}]}"#,
            r#"{"kinds":[{"kind":"x","endings":[".db"],"shapes":[]}]}"#,
            r#"{"kinds":[{"kind":"x","endings":["DB"],"shapes":[]}]}"#,
            r#"{"kinds":[{"kind":"x","endings":["db"],"shapes":["nope"]}]}"#,
            r#"{"kinds":[{"kind":"x","endings":["db"],"shapes":[],"app":"Outlook"}]}"#,
            r#"{"kinds":[{"kind":"x","endings":["db"],"shapes":[],"sidecars":["wal"]}]}"#,
            r#"{"kinds":[{"kind":"x","endings":["db"],"shapes":[],"sidecars":["-../x"]}]}"#,
            "not json",
        ] {
            assert!(LiveKinds::parse(bad).is_err(), "{bad}");
        }
        assert!(LiveKinds::parse(r#"{"kinds":[]}"#).is_ok());
    }
}
