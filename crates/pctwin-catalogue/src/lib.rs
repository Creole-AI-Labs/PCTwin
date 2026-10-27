//! Catalogues: the examples PCTwin knows, as data (Phase 2 adoption, changes 7 and 8).
//!
//! PCTwin works from what it **finds** on the real laptop, by general rules per mechanism. A
//! catalogue never decides what is found or what moves: its rows only **enrich** what was found
//! (a friendly name, a better route, a known exception, a translation between systems). So:
//!
//! - [`Catalogue::read`] is the only way to match found things against rows, and it always
//!   returns both halves ([`Reading`]): what a row knows, and what was found with no row yet
//!   ("found, not yet supported"). A reader cannot return only the matches.
//! - Every row is checked on load ([`Row::check`]); a row that cannot be read (an unknown kind,
//!   a bad value) is skipped and reported ([`Report`]), never the whole file. Fields this version
//!   does not know are reported and ignored.
//! - A catalogue compiled into the app ([`Catalogue::shipped`]) is covered by the app's own
//!   signature. One delivered later ([`Catalogue::signed`]) must carry a minisign (Ed25519)
//!   signature by a trusted key, bound to its name and version, and is never older than the
//!   highest version of it PCTwin ever accepted: that rollback mark is kept by PCTwin itself
//!   ([`RollbackMarks`]; the system's key store, beside the journal's key), never passed in by a
//!   caller. Rows that reach the admin helper ([`Trust::Privileged`]) need a privileged key.
//! - A file that gives one key twice in any object (which reader's value would win?) is refused
//!   whole; a row given twice, or two rows naming the same thing ([`Row::identity`]), are
//!   skipped and reported.
//! - Every file carries a format number under the shared format rule ([`CATALOGUE_FORMAT`]).
//!
//! **The production key is not here.** Catalogues delivered after release go through TUF
//! (`tough`, Security Design "Updates"); which keys sign them is decided there.
//! [`TrustedKeys::production`] is that seam: it holds no key today, so a production build
//! trusts only the catalogues compiled into it.

use pctwin_types::format::{Format, FormatError};
use serde::de::DeserializeOwned;
use serde_json::Value;

#[cfg(any(test, feature = "test-key"))]
pub mod test_key;

/// The catalogue file format this app reads and writes.
pub const CATALOGUE_FORMAT: Format = Format {
    name: "catalogue",
    current: 1,
    readable: &[],
};

/// The largest catalogue file read, in bytes.
pub const MAX_BYTES: usize = 4 * 1024 * 1024;
/// The most rows one catalogue may hold.
pub const MAX_ROWS: usize = 50_000;
/// The most problems reported per catalogue (the rest are counted).
const MAX_REPORTED: usize = 1_000;

/// How much trust a catalogue's rows need.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Trust {
    /// Rows that name, order or route what PCTwin found.
    Ordinary,
    /// Rows the admin helper acts on: a stricter key, and a minimum helper version.
    Privileged,
}

/// One kind of row. Each catalogue has one row type.
pub trait Row: DeserializeOwned {
    /// The catalogue's name: in its file, and bound into its signature.
    const CATALOGUE: &'static str;
    /// The fields this version knows. Others are reported, then ignored.
    const FIELDS: &'static [&'static str];
    /// The trust its rows need.
    const TRUST: Trust = Trust::Ordinary;
    /// Checks one row's values; `Err` skips that row with the reason.
    fn check(&self) -> Result<(), String> {
        Ok(())
    }
    /// What the row names, when two rows naming the same thing would contradict each other
    /// (the same catalog ID with two adapters): such rows are all skipped and reported, never
    /// the first silently winning.
    fn identity(&self) -> Option<String> {
        None
    }
}

/// The record of the highest version of each catalogue PCTwin has accepted (its rollback mark).
/// Kept by PCTwin where an older copy of the app's files cannot put it back (the system's key
/// store), and read and raised by [`Catalogue::signed`] itself.
///
/// What it protects against, and what not (fifth review, finding 7): it stops an older signed
/// catalogue coming back by itself, from a stale copy of the app's files or a mirror that serves
/// an old version. It does not stop a program already running as the person: that program can
/// remove the mark from the person's own key store (Windows' `cmdkey` does it, no administrator
/// needed) and then an older signed catalogue would be accepted again. Such a program can
/// already change the person's settings and files directly, so the mark is not a defence
/// against it; the signature still ensures every catalogue accepted is one PCTwin published.
pub trait RollbackMarks {
    /// The highest version of `catalogue` accepted so far (`None`: never one).
    fn highest(&self, catalogue: &str) -> Result<Option<u32>, MarkUnavailable>;
    /// Records that `version` of `catalogue` was accepted.
    fn raise(&self, catalogue: &str, version: u32) -> Result<(), MarkUnavailable>;
}

/// The rollback marks could not be read or written now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarkUnavailable;

/// Rollback marks in this process's memory (tests, and a run that keeps no delivered catalogue).
#[derive(Debug, Default)]
pub struct MemoryMarks {
    floor: u32,
    marks: std::sync::Mutex<std::collections::BTreeMap<String, u32>>,
}

impl MemoryMarks {
    pub fn new() -> Self {
        Self::default()
    }

    /// Marks where every catalogue has already been accepted at `version`.
    pub fn at(version: u32) -> Self {
        Self {
            floor: version,
            ..Self::default()
        }
    }
}

impl RollbackMarks for MemoryMarks {
    fn highest(&self, catalogue: &str) -> Result<Option<u32>, MarkUnavailable> {
        let m = self.marks.lock().map_err(|_| MarkUnavailable)?;
        let kept = m.get(catalogue).copied();
        Ok(match (kept, self.floor) {
            (Some(k), f) => Some(k.max(f)),
            (None, 0) => None,
            (None, f) => Some(f),
        })
    }
    fn raise(&self, catalogue: &str, version: u32) -> Result<(), MarkUnavailable> {
        let mut m = self.marks.lock().map_err(|_| MarkUnavailable)?;
        let e = m.entry(catalogue.to_string()).or_insert(version);
        *e = (*e).max(version);
        Ok(())
    }
}

/// Why a whole catalogue file was refused (a row's problem never refuses the file).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CatalogueError {
    #[error(transparent)]
    Format(#[from] FormatError),
    #[error("this catalogue is too large")]
    TooLarge,
    #[error("this catalogue is damaged: {0}")]
    Damaged(String),
    #[error("this is the {found:?} catalogue, not {want:?}")]
    WrongCatalogue { want: &'static str, found: String },
    #[error("this catalogue's signature is not from a trusted key")]
    NotSigned,
    #[error("this catalogue's signature is for another catalogue or version")]
    SignedForSomethingElse,
    #[error("this catalogue (version {found}) is older than the one in use ({in_use})")]
    Older { found: u32, in_use: u32 },
    #[error("these rows need a privileged key")]
    NeedsPrivilegedKey,
    #[error("the record of catalogue versions already accepted could not be read or written")]
    MarkUnavailable,
}

/// A row that could not be used, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skipped {
    /// Its place in the file's rows, from 0.
    pub row: usize,
    pub why: String,
}

/// What a catalogue's load could not use: rows skipped, and fields this version does not know.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    pub skipped: Vec<Skipped>,
    /// (row, field) for every unknown field (`row` is `None` for the file's own fields).
    pub unknown_fields: Vec<(Option<usize>, String)>,
    /// Problems beyond the first [`MAX_REPORTED`], counted only.
    pub more: usize,
    /// For a compiled-in catalogue that could not be read at all: why. Its rows are empty and
    /// everything found is "found, not yet supported"; nothing is gated on it.
    pub refused: Option<CatalogueError>,
}

impl Report {
    /// Nothing was skipped or unknown.
    pub fn is_clean(&self) -> bool {
        self.skipped.is_empty()
            && self.unknown_fields.is_empty()
            && self.more == 0
            && self.refused.is_none()
    }

    fn note(&mut self, f: impl FnOnce(&mut Self)) {
        if self.skipped.len() + self.unknown_fields.len() < MAX_REPORTED {
            f(self);
        } else {
            self.more += 1;
        }
    }
}

/// Where a catalogue came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Compiled into the app.
    Shipped,
    /// Delivered and checked against a trusted key.
    Signed,
}

/// One loaded catalogue.
#[derive(Debug, Clone)]
pub struct Catalogue<R> {
    version: u32,
    source: Source,
    rows: Vec<R>,
    report: Report,
}

/// What [`Catalogue::read`] gives: every found thing exactly once, either with the row that
/// knows it or as found with no row yet.
#[derive(Debug)]
pub struct Reading<'c, T, R> {
    pub known: Vec<(T, &'c R)>,
    /// Found on the laptop, with no row yet: still reported, never dropped.
    pub found_not_supported: Vec<T>,
}

/// The trusted comment a catalogue's signature must carry.
pub fn trusted_comment(catalogue: &str, version: u32) -> String {
    format!("pctwin-catalogue name={catalogue} version={version}")
}

/// Keys whose signatures are trusted, and for what.
#[derive(Debug, Clone)]
pub struct TrustedKeys {
    trust: Trust,
    keys: Vec<minisign_verify::PublicKey>,
}

impl TrustedKeys {
    /// The keys a production build trusts for delivered catalogues: none yet. Delivery through
    /// TUF (`tough`) decides them; until then only compiled-in catalogues are used.
    pub fn production() -> Self {
        Self {
            trust: Trust::Ordinary,
            keys: Vec::new(),
        }
    }

    /// Keys in minisign's public-key form (base64 of `Ed`, key ID, key).
    pub fn from_minisign_base64(trust: Trust, keys: &[&str]) -> Result<Self, CatalogueError> {
        let keys = keys
            .iter()
            .map(|k| minisign_verify::PublicKey::from_base64(k))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| CatalogueError::NotSigned)?;
        Ok(Self { trust, keys })
    }

    fn verify(&self, bytes: &[u8], signature: &str) -> Result<String, CatalogueError> {
        let sig =
            minisign_verify::Signature::decode(signature).map_err(|_| CatalogueError::NotSigned)?;
        // Prehashed signatures only (`allow_legacy` false).
        if self
            .keys
            .iter()
            .any(|k| k.verify(bytes, &sig, false).is_ok())
        {
            Ok(sig.trusted_comment().to_string())
        } else {
            Err(CatalogueError::NotSigned)
        }
    }
}

/// A JSON value read only to refuse one that gives a key twice in an object (a plain reader keeps
/// the last one silently; another reader might keep the first). The depth gate has run first.
struct NoKeyTwice;

impl<'de> serde::Deserialize<'de> for NoKeyTwice {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_any(NoKeyTwiceVisitor)
    }
}

struct NoKeyTwiceVisitor;

impl<'de> serde::de::Visitor<'de> for NoKeyTwiceVisitor {
    type Value = NoKeyTwice;
    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("JSON")
    }
    fn visit_bool<E>(self, _: bool) -> Result<NoKeyTwice, E> {
        Ok(NoKeyTwice)
    }
    fn visit_i64<E>(self, _: i64) -> Result<NoKeyTwice, E> {
        Ok(NoKeyTwice)
    }
    fn visit_u64<E>(self, _: u64) -> Result<NoKeyTwice, E> {
        Ok(NoKeyTwice)
    }
    fn visit_f64<E>(self, _: f64) -> Result<NoKeyTwice, E> {
        Ok(NoKeyTwice)
    }
    fn visit_str<E>(self, _: &str) -> Result<NoKeyTwice, E> {
        Ok(NoKeyTwice)
    }
    fn visit_unit<E>(self) -> Result<NoKeyTwice, E> {
        Ok(NoKeyTwice)
    }
    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut a: A) -> Result<NoKeyTwice, A::Error> {
        while a.next_element::<NoKeyTwice>()?.is_some() {}
        Ok(NoKeyTwice)
    }
    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut a: A) -> Result<NoKeyTwice, A::Error> {
        let mut seen = std::collections::BTreeSet::new();
        while let Some(k) = a.next_key::<String>()? {
            if !seen.insert(k.clone()) {
                return Err(serde::de::Error::custom(format!(
                    "the key {k:?} is given twice"
                )));
            }
            a.next_value::<NoKeyTwice>()?;
        }
        Ok(NoKeyTwice)
    }
}

fn parse<R: Row>(bytes: &[u8]) -> Result<(u32, Vec<R>, Report), CatalogueError> {
    if bytes.len() > MAX_BYTES {
        return Err(CatalogueError::TooLarge);
    }
    pctwin_types::nesting::check_bytes(bytes, pctwin_types::nesting::Syntax::Json)
        .map_err(|e| CatalogueError::Damaged(e.to_string()))?;
    CATALOGUE_FORMAT.check_json(bytes)?;
    serde_json::from_slice::<NoKeyTwice>(bytes)
        .map_err(|e| CatalogueError::Damaged(e.to_string()))?;
    let file: Value =
        serde_json::from_slice(bytes).map_err(|e| CatalogueError::Damaged(e.to_string()))?;
    let Some(top) = file.as_object() else {
        return Err(CatalogueError::Damaged("not an object".into()));
    };
    let name = top.get("catalogue").and_then(Value::as_str).unwrap_or("");
    if name != R::CATALOGUE {
        return Err(CatalogueError::WrongCatalogue {
            want: R::CATALOGUE,
            found: name.to_string(),
        });
    }
    let version = top
        .get("version")
        .and_then(Value::as_u64)
        .and_then(|v| u32::try_from(v).ok())
        .filter(|v| *v > 0)
        .ok_or_else(|| CatalogueError::Damaged("no version".into()))?;
    let Some(rows) = top.get("rows").and_then(Value::as_array) else {
        return Err(CatalogueError::Damaged("no rows".into()));
    };
    if rows.len() > MAX_ROWS {
        return Err(CatalogueError::TooLarge);
    }
    let mut report = Report::default();
    for key in top.keys() {
        if !["format", "catalogue", "version", "rows"].contains(&key.as_str()) {
            report.note(|r| r.unknown_fields.push((None, key.clone())));
        }
    }
    let mut out: Vec<(usize, R)> = Vec::with_capacity(rows.len());
    for (i, value) in rows.iter().enumerate() {
        if rows[..i].contains(value) {
            report.note(|r| {
                r.skipped.push(Skipped {
                    row: i,
                    why: "this row repeats one before it".into(),
                })
            });
            continue;
        }
        let Some(obj) = value.as_object() else {
            report.note(|r| {
                r.skipped.push(Skipped {
                    row: i,
                    why: "a row is an object".into(),
                })
            });
            continue;
        };
        for key in obj.keys() {
            if !R::FIELDS.contains(&key.as_str()) {
                report.note(|r| r.unknown_fields.push((Some(i), key.clone())));
            }
        }
        match R::deserialize(value) {
            Ok(row) => match row.check() {
                Ok(()) => out.push((i, row)),
                Err(why) => report.note(|r| r.skipped.push(Skipped { row: i, why })),
            },
            Err(e) => report.note(|r| {
                r.skipped.push(Skipped {
                    row: i,
                    why: e.to_string(),
                })
            }),
        }
    }
    // Two rows naming the same thing contradict each other: neither is used, both are reported.
    let ids: Vec<Option<String>> = out.iter().map(|(_, r)| r.identity()).collect();
    let mut kept = Vec::with_capacity(out.len());
    for (n, (i, row)) in out.into_iter().enumerate() {
        let twice = ids[n]
            .as_ref()
            .is_some_and(|id| ids.iter().filter(|x| x.as_ref() == Some(id)).count() > 1);
        if twice {
            report.note(|r| {
                r.skipped.push(Skipped {
                    row: i,
                    why: "another row names the same thing".into(),
                })
            });
        } else {
            kept.push(row);
        }
    }
    Ok((version, kept, report))
}

impl<R: Row> Catalogue<R> {
    /// A catalogue compiled into the app. Never fails and never panics: a file that cannot be
    /// read at all gives no rows, with the reason in [`Report::refused`] (everything found is
    /// then "found, not yet supported", which is honest and gates nothing).
    pub fn shipped(text: &str) -> Self {
        match parse::<R>(text.as_bytes()) {
            Ok((version, rows, report)) => Self {
                version,
                source: Source::Shipped,
                rows,
                report,
            },
            Err(e) => Self {
                version: 0,
                source: Source::Shipped,
                rows: Vec::new(),
                report: Report {
                    refused: Some(e),
                    ..Report::default()
                },
            },
        }
    }

    /// A delivered catalogue: its signature is checked first, before anything in it is read,
    /// against `keys`; the signature must name this catalogue and the version the file holds;
    /// the version must be at least the highest of this catalogue PCTwin ever accepted, read from
    /// `marks` (never rolled back), and once accepted that mark is raised to it. A mark that
    /// cannot be read or raised refuses the catalogue.
    pub fn signed(
        bytes: &[u8],
        signature: &str,
        keys: &TrustedKeys,
        marks: &dyn RollbackMarks,
    ) -> Result<Self, CatalogueError> {
        if R::TRUST > keys.trust {
            return Err(CatalogueError::NeedsPrivilegedKey);
        }
        if bytes.len() > MAX_BYTES {
            return Err(CatalogueError::TooLarge);
        }
        let comment = keys.verify(bytes, signature)?;
        let (version, rows, report) = parse::<R>(bytes)?;
        if comment != trusted_comment(R::CATALOGUE, version) {
            return Err(CatalogueError::SignedForSomethingElse);
        }
        let in_use = marks
            .highest(R::CATALOGUE)
            .map_err(|_| CatalogueError::MarkUnavailable)?
            .unwrap_or(0);
        if version < in_use {
            return Err(CatalogueError::Older {
                found: version,
                in_use,
            });
        }
        marks
            .raise(R::CATALOGUE, version)
            .map_err(|_| CatalogueError::MarkUnavailable)?;
        Ok(Self {
            version,
            source: Source::Signed,
            rows,
            report,
        })
    }

    /// The newer of two catalogues of the same kind (this one when they are the same version).
    pub fn newer(self, other: Self) -> Self {
        if other.version > self.version {
            other
        } else {
            self
        }
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn source(&self) -> Source {
        self.source
    }

    /// The rows that loaded.
    pub fn rows(&self) -> &[R] {
        &self.rows
    }

    /// What could not be used.
    pub fn report(&self) -> &Report {
        &self.report
    }

    /// Matches every found thing against the rows: the first row `matches` accepts knows it;
    /// one no row accepts is "found, not yet supported". Every found thing comes back once.
    pub fn read<T>(
        &self,
        found: impl IntoIterator<Item = T>,
        mut matches: impl FnMut(&T, &R) -> bool,
    ) -> Reading<'_, T, R> {
        let mut reading = Reading {
            known: Vec::new(),
            found_not_supported: Vec::new(),
        };
        for thing in found {
            match self.rows.iter().find(|r| matches(&thing, r)) {
                Some(row) => reading.known.push((thing, row)),
                None => reading.found_not_supported.push(thing),
            }
        }
        reading
    }
}

#[cfg(test)]
mod tests;
