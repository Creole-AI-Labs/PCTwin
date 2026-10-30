//! What the app and the parser process say to each other over the child's own pipes: a fixed
//! list of typed requests, never a shell, a path or a free-form command.
//!
//! Every message is a frame: one byte saying what it is, its body's length (4 bytes,
//! big-endian), then the body, never longer than [`MAX_FRAME`]. JSON bodies name no field the
//! types do not ([`serde`]'s `deny_unknown_fields`).
//!
//! - The parser starts by saying [`Hello`]: the protocol it speaks and how it is locked down.
//! - The app sends one [`Request`] at a time; it names the kind of reading and the file's size,
//!   never the file's name.
//! - While reading, the parser asks for pieces of the file ([`ReadAt`], at most [`MAX_READ`]
//!   bytes each); the app answers with the bytes, from the file it holds open, or says the read
//!   failed. The parser never opens a file itself.
//! - The parser ends each request with one [`Answer`].

use std::io::{self, Read, Write};

use pctwin_maker::{InstallerRead, OldCopy};
use pctwin_readiness::checks::{FontNames, Shape};
use pctwin_special::content::{ContentError, DbFacts, JsonFacts};
use serde::{Deserialize, Serialize};

/// The version of this protocol. A parser program speaking another is refused (a parser left
/// over from another build).
pub const PROTOCOL: u32 = 1;

/// Longest piece of a file asked for at once.
pub const MAX_READ: u32 = 1024 * 1024;
/// Longest JSON body.
pub const MAX_JSON: usize = 4 * 1024 * 1024;
/// Longest frame either way.
pub const MAX_FRAME: usize = MAX_JSON;

/// App to parser: a [`Request`].
pub const TAG_REQUEST: u8 = 1;
/// App to parser: the bytes asked for (fewer only at the end of the file).
pub const TAG_BYTES: u8 = 2;
/// App to parser: the piece asked for could not be read.
pub const TAG_READ_FAILED: u8 = 3;
/// Parser to app: [`Hello`].
pub const TAG_HELLO: u8 = 10;
/// Parser to app: a [`ReadAt`].
pub const TAG_READ_AT: u8 = 11;
/// Parser to app: an [`Answer`].
pub const TAG_ANSWER: u8 = 12;
/// Parser to app: a piece of what it hands back (at most [`MAX_READ`] bytes), before its answer.
pub const TAG_PIECE: u8 = 13;

/// What the parser says first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hello {
    pub protocol: u32,
    /// How the parser process is locked down on this system, in its own words
    /// ([`crate::lockdown::Confinement`]).
    pub confinement: crate::lockdown::Confinement,
}

/// One reading of one file, of `size` bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "read", deny_unknown_fields)]
pub enum Request {
    /// An installer's kind, build and signer (the maker check).
    Installer { size: u64 },
    /// The maker and build of the old laptop's own copy of a Windows program or package.
    OldCopy { size: u64 },
    /// Whether a file is a font PCTwin reads, and its family names.
    Font { size: u64 },
    /// Whether a ZIP archive's index is whole.
    Archive { size: u64 },
    /// Whether a SQLite database's header is sound.
    Database { size: u64 },
    /// A JSON file's facts (2.5).
    Json { size: u64 },
    /// A database's tables, kind and linked paths (2.5): `size` bytes of the database, then
    /// `log` bytes of its write-ahead log; the scan of linked paths stops after `millis`.
    DatabaseFacts { size: u64, log: u64, millis: u64 },
    /// One complete database from `size` bytes of a database and `log` bytes of its log, kept
    /// only if SQLite's `quick_check` passes (2.5's capture). Its bytes come back in
    /// [`TAG_PIECE`] frames before the answer.
    DatabaseSnapshot { size: u64, log: u64 },
    /// Whether a `manifest.json` is a WebExtension's.
    AddOn { size: u64 },
    /// One saved Apple Mail message's facts (2.5 mail).
    Emlx { size: u64 },
    /// Which kind of account each Apple Mail account folder is (2.5 mail): `size` bytes of the
    /// received mail index, then `log` bytes of its write-ahead log.
    MailKinds { size: u64, log: u64 },
    /// What a browser's file holds (2.5 browsers): `size` bytes of the file, then `second` bytes
    /// of a second one (a database's log, or a manifest's messages; none when 0).
    Browser {
        size: u64,
        second: u64,
        what: pctwin_special::browser_read::BrowserRead,
    },
    /// A game launcher's install record, as a tree of names and texts (2.5 games).
    GameRecord {
        size: u64,
        format: pctwin_special::gamefmt::RecordFormat,
    },
    /// The entries a save list in the Ludusavi manifest format holds for some games (2.5
    /// games): `wanted` bytes of the games asked about (JSON), then `size` bytes of the list.
    SaveManifest { wanted: u64, size: u64 },
    /// The value under one key of a database's key-value table (2.4 recent lists): `wanted`
    /// bytes of which ([`pctwin_special::stored::StoredKey`], JSON), then `size` bytes of the
    /// database.
    StoredValue { wanted: u64, size: u64 },
    /// The database with one key's value replaced (2.4 recent lists): `wanted` bytes of the
    /// change ([`pctwin_special::stored::StoredChange`], JSON), then `size` bytes of the
    /// database. Its bytes come back in [`TAG_PIECE`] frames before the answer.
    ReplaceValue { wanted: u64, size: u64 },
}

impl Request {
    pub fn size(&self) -> u64 {
        match *self {
            Request::Installer { size }
            | Request::OldCopy { size }
            | Request::Font { size }
            | Request::Archive { size }
            | Request::Database { size }
            | Request::Json { size }
            | Request::AddOn { size }
            | Request::Emlx { size }
            | Request::GameRecord { size, .. } => size,
            Request::SaveManifest { wanted, size }
            | Request::StoredValue { wanted, size }
            | Request::ReplaceValue { wanted, size } => wanted.saturating_add(size),
            Request::DatabaseFacts { size, log, .. }
            | Request::DatabaseSnapshot { size, log }
            | Request::MailKinds { size, log } => size.saturating_add(log),
            Request::Browser { size, second, .. } => size.saturating_add(second),
        }
    }

    /// The most bytes the parser may hand back in [`TAG_PIECE`] frames.
    pub fn most_back(&self) -> u64 {
        match *self {
            Request::DatabaseSnapshot { .. } | Request::ReplaceValue { .. } => {
                pctwin_special::content::MAX_DB
            }
            _ => 0,
        }
    }

    /// Time the request itself asks for, on top of reading its bytes.
    pub fn own_time(&self) -> std::time::Duration {
        match *self {
            Request::DatabaseFacts { millis, .. } => {
                std::time::Duration::from_millis(millis.min(10 * 60 * 1000))
            }
            _ => std::time::Duration::ZERO,
        }
    }
}

/// A piece of the file the parser needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadAt {
    pub offset: u64,
    pub len: u32,
}

/// The parser's answer to one [`Request`], of the same kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum Answer {
    Installer(InstallerRead),
    OldCopy(OldCopy),
    Font(FontNames),
    Archive(Shape),
    Database(Shape),
    Json(Result<JsonFacts, ContentError>),
    DatabaseFacts(Result<DbFacts, ContentError>),
    /// The database's length, its bytes having come in [`TAG_PIECE`] frames.
    DatabaseSnapshot(Result<u64, ContentError>),
    AddOn(bool),
    Emlx(Result<pctwin_special::mail::EmlxFacts, ContentError>),
    MailKinds(Result<Vec<pctwin_special::applemail::AccountScheme>, ContentError>),
    Browser(Result<pctwin_special::browser_read::Found, ContentError>),
    GameRecord(Result<pctwin_special::gamefmt::Tree, ContentError>),
    SaveManifest(Result<pctwin_special::gamefmt::ManifestRead, ContentError>),
    StoredValue(Result<Option<String>, ContentError>),
    /// The database's length, its bytes having come in [`TAG_PIECE`] frames.
    ReplaceValue(Result<u64, ContentError>),
    /// The reading could not finish (a piece of the file could not be read): why.
    Failed(String),
}

impl Answer {
    /// Whether this answers `request` (an answer of another kind is refused).
    pub fn answers(&self, request: &Request) -> bool {
        matches!(
            (self, request),
            (Answer::Installer(_), Request::Installer { .. })
                | (Answer::OldCopy(_), Request::OldCopy { .. })
                | (Answer::Font(_), Request::Font { .. })
                | (Answer::Archive(_), Request::Archive { .. })
                | (Answer::Database(_), Request::Database { .. })
                | (Answer::Json(_), Request::Json { .. })
                | (Answer::DatabaseFacts(_), Request::DatabaseFacts { .. })
                | (
                    Answer::DatabaseSnapshot(_),
                    Request::DatabaseSnapshot { .. }
                )
                | (Answer::AddOn(_), Request::AddOn { .. })
                | (Answer::Emlx(_), Request::Emlx { .. })
                | (Answer::MailKinds(_), Request::MailKinds { .. })
                | (Answer::Browser(_), Request::Browser { .. })
                | (Answer::GameRecord(_), Request::GameRecord { .. })
                | (Answer::SaveManifest(_), Request::SaveManifest { .. })
                | (Answer::StoredValue(_), Request::StoredValue { .. })
                | (Answer::ReplaceValue(_), Request::ReplaceValue { .. })
                | (Answer::Failed(_), _)
        )
    }
}

/// Writes one frame.
pub fn write_frame(w: &mut impl Write, tag: u8, body: &[u8]) -> io::Result<()> {
    if body.len() > MAX_FRAME {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "frame too long",
        ));
    }
    let len = u32::try_from(body.len()).map_err(|_| io::Error::other("frame too long"))?;
    let mut head = [0u8; 5];
    head[0] = tag;
    head[1..].copy_from_slice(&len.to_be_bytes());
    w.write_all(&head)?;
    w.write_all(body)?;
    w.flush()
}

/// Reads one frame of at most `limit` bytes. `Ok(None)`: the other side closed between frames.
pub fn read_frame(r: &mut impl Read, limit: usize) -> io::Result<Option<(u8, Vec<u8>)>> {
    let mut head = [0u8; 5];
    let mut got = 0;
    while got < head.len() {
        match r.read(&mut head[got..]) {
            Ok(0) if got == 0 => return Ok(None),
            Ok(0) => return Err(io::ErrorKind::UnexpectedEof.into()),
            Ok(n) => got += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    let len = u32::from_be_bytes([head[1], head[2], head[3], head[4]]) as usize;
    if len > limit {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "frame longer than allowed",
        ));
    }
    let mut body = vec![0u8; len];
    r.read_exact(&mut body)?;
    Ok(Some((head[0], body)))
}

/// Writes `value` as a JSON frame.
pub fn write_json<T: Serialize>(w: &mut impl Write, tag: u8, value: &T) -> io::Result<()> {
    let body = serde_json::to_vec(value).map_err(io::Error::other)?;
    write_frame(w, tag, &body)
}

/// Reads a JSON body, refusing anything else.
pub fn json<T: for<'de> Deserialize<'de>>(body: &[u8]) -> io::Result<T> {
    serde_json::from_slice(body).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}
