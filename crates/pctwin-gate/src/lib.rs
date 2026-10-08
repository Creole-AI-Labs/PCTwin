//! PCTwin safety gate (Security Design Part B): the new laptop trusts nothing that arrives.
//!
//! - [`IncomingPath::parse`] checks every path the old laptop sends. Paths are relative to an
//!   approved folder, use `/` between folders, and anything that could climb out (`..`, an
//!   absolute path, empty or `.` parts, NUL) is refused. Sizes and depth are capped. Names are
//!   held in one Unicode form (NFC), so the same accented name never appears twice.
//! - [`convert_name`] makes a name storable on the target system (Engineering Plan 9.6). On
//!   Windows: forbidden characters become lookalikes, names that look like short-name aliases
//!   (`LONGFO~1`) are renamed, over-long names are shortened, trailing dots and spaces are trimmed
//!   and reserved names such as `CON` are renamed, repeating until the name is stable. Every
//!   change is reported as a [`NameChange`], and the name as sent is kept for the report.
//! - [`Destination`] writes only inside an approved folder, through a cap-std directory handle, so
//!   a link cannot redirect a write elsewhere. A file is written under a temporary `.pctwin-` name and
//!   gets its real name only once every announced byte has arrived; an unfinished file is removed.
//!   A big file sent over several lanes is written in sections at their place
//!   ([`IncomingFile::write_at`]); the gate keeps exactly which bytes arrived, refuses anything
//!   outside the announced size or written twice, and can reserve the whole size first
//!   ([`IncomingFile::reserve`]).
//!   Nothing is overwritten: a taken name becomes `name (2).ext`, found in constant time even when
//!   thousands of files share a name.
//! - [`Destinations`] is the table of approved places (each person's folders, a shared folder, a
//!   chosen drive, an offload drive). The old laptop can only name an entry; an unknown label is
//!   refused, labels are never used as paths, and another person's account is opened only through
//!   the admin helper.
//!
//! Received files are data: nothing here runs, opens or interprets their contents.

use std::collections::{BTreeMap, HashMap};
use std::hash::{BuildHasher, RandomState};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};

use cap_std::ambient_authority;
pub use cap_std::fs::Dir;
use cap_std::fs::{File, OpenOptions};
use unicode_normalization::UnicodeNormalization;

/// Longest single file or folder name, in bytes (the limit on every supported system).
pub const MAX_COMPONENT_BYTES: usize = 255;
/// Deepest folder nesting accepted.
pub const MAX_DEPTH: usize = 128;
/// Longest whole path accepted, in bytes.
pub const MAX_PATH_BYTES: usize = 4096;
/// Most alternative names tried for one file before giving up on a clash.
const MAX_CLASH_ATTEMPTS: u32 = 100_000;
/// An extension longer than this is treated as part of the name when shortening or numbering.
const MAX_EXTENSION_BYTES: usize = 16;
/// Longest stem treated as a possible short-name alias.
const MAX_ALIAS_STEM_CHARS: usize = 16;
/// Longest clash number added to a name: ` (4294967295)`.
const MAX_NUMBER_BYTES: usize = 13;
/// Most separate parts of one file at a time. Sections of a move stay far below this (blocks are
/// at least 128 KiB and parts that touch are joined); it stops scattered tiny writes filling memory.
pub const MAX_FILE_PARTS: usize = 4096;
/// Largest file any supported system stores (signed 64-bit file offsets).
const MAX_FILE_BYTES: u64 = i64::MAX as u64;
/// Most clash hints remembered; past this they are forgotten and rebuilt.
const MAX_CLASH_HINTS: usize = 1 << 20;

/// Why a path from the old laptop was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PathError {
    #[error("the path is empty")]
    Empty,
    #[error("the path starts at the top of a drive instead of inside the approved folder")]
    Absolute,
    #[error("the path tries to go above the approved folder")]
    Traversal,
    #[error("the path has an empty part")]
    EmptyComponent,
    #[error("the path has a '.' part")]
    DotComponent,
    #[error("the path contains a NUL character")]
    NulCharacter,
    #[error("a file or folder name is too long")]
    ComponentTooLong,
    #[error("the folders are nested too deeply")]
    TooDeep,
    #[error("the path is too long")]
    TooLong,
}

/// A path from the old laptop that stays inside the approved folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncomingPath {
    original: String,
    components: Vec<String>,
}

impl IncomingPath {
    /// Checks a `/`-separated path relative to the approved folder.
    pub fn parse(path: &str) -> Result<Self, PathError> {
        if path.is_empty() {
            return Err(PathError::Empty);
        }
        if path.contains('\0') {
            return Err(PathError::NulCharacter);
        }
        if path.starts_with('/') {
            return Err(PathError::Absolute);
        }
        if path.len() > MAX_PATH_BYTES {
            return Err(PathError::TooLong);
        }
        let mut components = Vec::new();
        for part in path.split('/') {
            match part {
                "" => return Err(PathError::EmptyComponent),
                "." => return Err(PathError::DotComponent),
                ".." => return Err(PathError::Traversal),
                _ => {}
            }
            let composed: String = part.nfc().collect();
            if composed.len() > MAX_COMPONENT_BYTES {
                return Err(PathError::ComponentTooLong);
            }
            components.push(composed);
            if components.len() > MAX_DEPTH {
                return Err(PathError::TooDeep);
            }
        }
        Ok(Self {
            original: path.to_string(),
            components,
        })
    }

    /// The folder and file names as the old laptop sent them (composed), outermost first. These
    /// are not yet safe to use as names on this system: write through [`Destination`].
    pub fn components(&self) -> &[String] {
        &self.components
    }

    /// The path exactly as the old laptop sent it, for the report and a later move back.
    pub fn original(&self) -> &str {
        &self.original
    }
}

/// A system names are converted for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    MacOs,
    Linux,
}

impl Platform {
    /// The system this program runs on.
    pub fn host() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::MacOs
        } else {
            Self::Linux
        }
    }
}

/// A change made to a name so it could be stored, for the report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameChange {
    /// Characters the system forbids were replaced with lookalikes.
    ForbiddenCharacters,
    /// A name the system reserves (such as `CON`) was renamed.
    ReservedName,
    /// A name that looks like a Windows short-name alias (such as `LONGFO~1`) was renamed, so it
    /// cannot land inside an existing folder or file with a longer name.
    ShortNameAlias,
    /// Trailing dots or spaces were removed.
    TrailingDotsOrSpaces,
    /// The name was shortened to fit.
    Shortened,
    /// The name was already taken, so a number was added.
    NameClash,
}

/// A name ready to store, and what was changed to get there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Converted {
    pub name: String,
    pub changes: Vec<NameChange>,
}

/// Makes `name` storable on `platform`. The name is stored composed (NFC); other changes are
/// reported.
pub fn convert_name(name: &str, platform: Platform) -> Converted {
    let mut name: String = name.nfc().collect();
    let mut changes = Vec::new();
    let note = |c: NameChange, changes: &mut Vec<NameChange>| {
        if !changes.contains(&c) {
            changes.push(c);
        }
    };
    let windows = platform == Platform::Windows;
    if windows {
        // No later step adds a forbidden character, so this runs once.
        let replaced: String = name.chars().map(windows_safe_char).collect();
        if replaced != name {
            note(NameChange::ForbiddenCharacters, &mut changes);
            name = replaced;
        }
    }
    // Each step can undo another (shortening can expose a trailing space, trimming can expose a
    // reserved name or an alias), so every step runs on every pass until nothing changes. Marks go
    // right after the stem, near the start, where shortening never cuts them off, so this settles
    // within a few passes.
    for _ in 0..8 {
        let before = name.clone();
        if name.len() > MAX_COMPONENT_BYTES {
            name = shorten(&name, MAX_COMPONENT_BYTES);
            note(NameChange::Shortened, &mut changes);
        }
        if windows {
            let trimmed = name.trim_end_matches(['.', ' ']);
            if trimmed.len() != name.len() {
                note(NameChange::TrailingDotsOrSpaces, &mut changes);
                name = if trimmed.is_empty() {
                    "_".to_string()
                } else {
                    trimmed.to_string()
                };
            }
            if is_windows_reserved(&name) {
                note(NameChange::ReservedName, &mut changes);
                name = mark_after_stem(&name);
            }
            if looks_like_short_name(&name) {
                note(NameChange::ShortNameAlias, &mut changes);
                name = mark_after_stem(&name);
            }
        }
        if name.is_empty() {
            name = "_".to_string();
        }
        if name == before {
            break;
        }
    }
    Converted { name, changes }
}

/// Windows forbids `< > : " / \ | ? *` and control characters in names.
fn windows_safe_char(c: char) -> char {
    match c {
        '<' => '\u{FF1C}',
        '>' => '\u{FF1E}',
        ':' => '\u{FF1A}',
        '"' => '\u{FF02}',
        '/' => '\u{FF0F}',
        '\\' => '\u{FF3C}',
        '|' => '\u{FF5C}',
        '?' => '\u{FF1F}',
        '*' => '\u{FF0A}',
        c if (c as u32) < 32 => '_',
        c => c,
    }
}

/// The part before the first dot, and the rest.
fn split_stem(name: &str) -> (&str, &str) {
    match name.find('.') {
        Some(i) => (&name[..i], &name[i..]),
        None => (name, ""),
    }
}

/// The stem without the trailing spaces Windows ignores when it looks a name up.
fn checked_stem(name: &str) -> &str {
    split_stem(name).0.trim_end_matches(' ')
}

/// Adds `_` right after the stem's text: `CON.txt` → `CON_.txt`, `CON  .txt` → `CON_  .txt`.
fn mark_after_stem(name: &str) -> String {
    let at = checked_stem(name).len();
    format!("{}_{}", &name[..at], &name[at..])
}

/// `CON`, `PRN`, `AUX`, `NUL`, `CONIN$`, `CONOUT$`, `COM0`–`COM9`, `LPT0`–`LPT9` (and the
/// superscript-digit forms), whatever the case and extension. Windows ignores trailing spaces in
/// the stem when it checks.
fn is_windows_reserved(name: &str) -> bool {
    let stem = checked_stem(name).to_uppercase();
    if matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) {
        return true;
    }
    let mut chars = stem.chars();
    let prefix: String = chars.by_ref().take(3).collect();
    let rest: Vec<char> = chars.collect();
    (prefix == "COM" || prefix == "LPT")
        && rest.len() == 1
        && matches!(rest[0], '0'..='9' | '\u{B9}' | '\u{B2}' | '\u{B3}')
}

/// `LONGFO~1`, `PROGRA~2.TXT`: up to six characters, a tilde and digits, an extension of up to
/// three. Windows may resolve such a name to an existing long-named file or folder. Real aliases
/// are at most eight characters; up to [`MAX_ALIAS_STEM_CHARS`] are caught, to be safe while
/// keeping the mark near the start of the name.
fn looks_like_short_name(name: &str) -> bool {
    let stem = checked_stem(name);
    let (_, rest) = split_stem(name);
    let ext = rest.strip_prefix('.').unwrap_or(rest);
    let Some((base, digits)) = stem.rsplit_once('~') else {
        return false;
    };
    stem.chars().count() <= MAX_ALIAS_STEM_CHARS
        && (1..=6).contains(&base.chars().count())
        && !digits.is_empty()
        && digits.chars().all(|c| c.is_ascii_digit())
        && ext.chars().count() <= 3
        && !ext.contains('.')
}

/// The name split for shortening and numbering: a short final extension is kept whole.
fn split_extension(name: &str) -> (&str, &str) {
    match name.rfind('.') {
        Some(i) if i > 0 && name.len() - i <= MAX_EXTENSION_BYTES => (&name[..i], &name[i..]),
        _ => (name, ""),
    }
}

/// Shortens `name` to at most `max` bytes, keeping its extension and whole characters.
fn shorten(name: &str, max: usize) -> String {
    let (stem, ext) = split_extension(name);
    format!("{}{ext}", cut_to(stem, max.saturating_sub(ext.len())))
}

fn cut_to(s: &str, max: usize) -> &str {
    let mut cut = max.min(s.len());
    while !s.is_char_boundary(cut) {
        cut -= 1;
    }
    &s[..cut]
}

/// Why a file could not be written.
#[derive(Debug, thiserror::Error)]
pub enum GateError {
    #[error(transparent)]
    Path(#[from] PathError),
    #[error("a file is in the way of a folder with the same name")]
    Conflict,
    #[error("too many files already use this name")]
    TooManyClashes,
    #[error("the file should be {announced} bytes but {received} arrived")]
    SizeMismatch { announced: u64, received: u64 },
    #[error("that place on the new laptop was not approved")]
    UnknownDestination,
    #[error("that place on the new laptop is already approved")]
    DuplicateDestination,
    #[error("a place's label must be 1 to 64 plain letters, numbers, '-', '_' or '.'")]
    BadDestinationId,
    #[error("another person's account can only be opened through the administrator helper")]
    NeedsAdminHelper,
    #[error("the old laptop sent data for a place outside the file it announced")]
    OutsideFile,
    #[error("the old laptop sent part of a file again")]
    Overlap,
    #[error("the old laptop sent a file in too many scattered pieces")]
    TooScattered,
    #[error("there is not enough free space on the new laptop for this file")]
    NoSpace,
    #[error("a temporary name's tag must be 1 to 64 of a-z, 0-9 and '-'")]
    BadTag,
    #[error("this drive cannot give the file its name without risking another file")]
    NoSafeName,
    #[error("writing failed: {0}")]
    Io(#[from] io::Error),
}

/// An approved folder on the new laptop. Everything written goes inside it.
pub struct Destination {
    root: Dir,
    /// Where it is, when it was opened by its path (not through the admin helper): needed only to
    /// hand an undone file to the system Trash, which takes paths.
    root_path: Option<std::path::PathBuf>,
    /// Where to start numbering the next clash of a name in a folder, so thousands of clashes
    /// stay fast. Only a hint: the exact name is always tried first and every name is claimed
    /// without replacing anything, so a wrong or shared hint can only skip numbers.
    clash_hints: Mutex<ClashHints>,
}

/// Clash hints keyed by a hash of the folder and name, folded so every spelling a disk might treat
/// as the same lands on one key. Entries exist only for names that clashed, and are capped.
#[derive(Default)]
struct ClashHints {
    hasher: RandomState,
    next: HashMap<u64, u32>,
}

impl ClashHints {
    fn key(&self, folder: &str, name: &str) -> u64 {
        let fold = |s: &str| -> String {
            s.chars()
                .flat_map(char::to_uppercase)
                .flat_map(char::to_lowercase)
                .collect()
        };
        self.hasher.hash_one((fold(folder), fold(name)))
    }

    fn remember(&mut self, key: u64, next: u32) {
        if self.next.len() >= MAX_CLASH_HINTS {
            self.next.clear();
        }
        self.next.insert(key, next);
    }
}

/// Distinguishes temporary names created by this process.
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

impl Destination {
    /// Opens the approved folder at `path`. This is the only place an ordinary path is used.
    pub fn open(path: &Path) -> io::Result<Self> {
        let mut dest = Self::from_dir(Dir::open_ambient_dir(path, ambient_authority())?);
        dest.root_path = Some(path.to_path_buf());
        Ok(dest)
    }

    fn from_dir(root: Dir) -> Self {
        Self {
            root,
            root_path: None,
            clash_hints: Mutex::new(ClashHints::default()),
        }
    }

    /// Starts receiving a file at `path` (inside the approved folder) that must be exactly
    /// `announced` bytes. It is written under a temporary `.pctwin-` name; [`IncomingFile::finish`]
    /// gives it its real name.
    pub fn create_file<'d>(
        &'d self,
        path: &IncomingPath,
        announced: u64,
    ) -> Result<IncomingFile<'d>, GateError> {
        self.create_with(path, announced, None)
    }

    /// As [`create_file`](Self::create_file), under the temporary name [`temp_name`]`(tag)`, which
    /// the change journal chose and recorded before the file exists, so after a crash it knows
    /// exactly which file is its own. `tag` is 1 to 64 of `a`-`z`, `0`-`9` and `-`. A file
    /// already under that name is never reused or replaced: the start is refused.
    pub fn create_file_tagged<'d>(
        &'d self,
        path: &IncomingPath,
        announced: u64,
        tag: &str,
    ) -> Result<IncomingFile<'d>, GateError> {
        if !is_plain_tag(tag) {
            return Err(GateError::BadTag);
        }
        self.create_with(path, announced, Some(tag))
    }

    fn create_with<'d>(
        &'d self,
        path: &IncomingPath,
        announced: u64,
        tag: Option<&str>,
    ) -> Result<IncomingFile<'d>, GateError> {
        let host = Platform::host();
        let (file_name, folders) = path
            .components()
            .split_last()
            .ok_or(GateError::Path(PathError::Empty))?;
        let mut changes = Vec::new();
        let mut shown = Vec::new();
        let mut created = Vec::new();
        let mut dir = self.root.try_clone()?;
        for folder in folders {
            let converted = convert_name(folder, host);
            changes.extend(converted.changes.iter().copied());
            let (opened, made) = open_or_create_folder(&dir, &converted.name)?;
            dir = opened;
            shown.push(converted.name);
            if made {
                created.push(shown.join("/"));
            }
        }
        let converted = convert_name(file_name, host);
        changes.extend(converted.changes.iter().copied());
        let (file, temp_name) = match tag {
            Some(tag) => {
                let name = temp_name(tag);
                let mut options = OpenOptions::new();
                options.read(true).write(true).create_new(true);
                (dir.open_with(&name, &options)?, name)
            }
            None => create_temp_file(&dir)?,
        };
        Ok(IncomingFile {
            destination: self,
            dir,
            file: Some(file),
            temp_name: Some(temp_name),
            announced,
            received: 0,
            cursor: 0,
            arrived: BTreeMap::new(),
            folder: shown.join("/"),
            name: converted.name,
            sent_path: path.original().to_string(),
            changes,
            modified: None,
            created,
        })
    }

    /// The folder (inside the approved folder, `/` between folders) a file sent as `path` lands
    /// in, with names converted exactly as for writing. Nothing on disk is looked at.
    pub fn folder_of(&self, path: &IncomingPath) -> String {
        let host = Platform::host();
        let parts = path.components();
        parts[..parts.len().saturating_sub(1)]
            .iter()
            .map(|f| convert_name(f, host).name)
            .collect::<Vec<_>>()
            .join("/")
    }

    /// The size of the regular file at the stored path `stored`, or `None` if there is none (a
    /// missing folder, or a folder or link in its place). Never follows a link or creates
    /// anything.
    pub fn look(&self, stored: &str) -> io::Result<Option<u64>> {
        let Some((dir, name)) = self.open_stored_folder(stored)? else {
            return Ok(None);
        };
        match dir.symlink_metadata(name) {
            Ok(meta) if meta.is_file() => Ok(Some(meta.len())),
            Ok(_) => Ok(None),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Removes PCTwin's temporary file at the stored path `stored`. Only a regular file with a
    /// temporary `.pctwin-` name is ever removed; any other name is refused. Returns whether a
    /// file was removed.
    pub fn remove_temp(&self, stored: &str) -> io::Result<bool> {
        let Some((dir, name)) = self.open_stored_folder(stored)? else {
            return Ok(false);
        };
        if !is_temp_name(name) {
            return Err(invalid("not a PCTwin temporary file"));
        }
        match dir.symlink_metadata(name) {
            Ok(meta) if meta.is_file() => {}
            Ok(_) => return Ok(false),
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(e) => return Err(e),
        }
        match dir.remove_file(name) {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// The regular file at the stored path `stored`: its size, modified time and identity, or
    /// `None` if there is none. Never follows a link.
    pub fn stat(&self, stored: &str) -> io::Result<Option<Stat>> {
        let Some((dir, name)) = self.open_stored_folder(stored)? else {
            return Ok(None);
        };
        match dir.symlink_metadata(name) {
            Ok(meta) if meta.is_file() => {}
            Ok(_) => return Ok(None),
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        }
        let file = dir.open(name)?.into_std();
        let meta = file.metadata()?;
        Ok(Some(Stat {
            len: meta.len(),
            modified: meta.modified().ok(),
            id: identity(&file)?.0,
        }))
    }

    /// The identity of the folder at the stored path `stored` (`""` is the approved folder
    /// itself), or `None` if there is no folder there. Never follows a link.
    pub fn folder_identity(&self, stored: &str) -> io::Result<Option<FileId>> {
        let dir = if stored.is_empty() {
            self.root.try_clone()?
        } else {
            let Some((parent, name)) = self.open_stored_folder(stored)? else {
                return Ok(None);
            };
            match parent.symlink_metadata(name) {
                Ok(meta) if meta.is_dir() => parent.open_dir(name)?,
                Ok(_) => return Ok(None),
                Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
                Err(e) => return Err(e),
            }
        };
        Ok(Some(identity(&dir.into_std_file())?.0))
    }

    /// Moves the file at the stored path `from` to the stored path `to` in the same place (making
    /// `to`'s folders if needed), by handle, only if the file at `from` is still the very file
    /// `expect`, and never replacing anything. Where the drive cannot move without replacing, a
    /// hard link then removing the old name does the same; where it can do neither, nothing moves.
    pub fn move_file(&self, from: &str, to: &str, expect: FileId) -> Result<Moved, GateError> {
        let Some((from_dir, from_name)) = self.open_stored_folder(from)? else {
            return Ok(Moved::NotThatFile);
        };
        let still = from_dir
            .symlink_metadata(from_name)
            .is_ok_and(|meta| meta.is_file())
            && from_dir
                .open(from_name)
                .and_then(|f| identity(&f.into_std()))
                .is_ok_and(|(id, _)| id == expect);
        if !still {
            return Ok(Moved::NotThatFile);
        }
        let to_parts = stored_parts(to)?;
        let (to_name, to_folders) = to_parts.split_last().ok_or_else(|| invalid("empty path"))?;
        let mut to_dir = self.root.try_clone()?;
        for folder in to_folders {
            to_dir = open_or_create_folder(&to_dir, folder)?.0;
        }
        let from_folder = from.rsplit_once('/').map_or("", |(f, _)| f);
        let to_folder = to.rsplit_once('/').map_or("", |(f, _)| f);
        let from_path = || self.folder_path(&from_dir, from_folder);
        let to_path = || self.folder_path(&to_dir, to_folder);
        let moved =
            match move_no_replace(&from_dir, from_name, &to_dir, to_name, &from_path, &to_path) {
                Ok(moved) => moved,
                Err(GateError::NoSafeName) => match from_dir.hard_link(from_name, &to_dir, to_name)
                {
                    Ok(()) => {
                        from_dir.remove_file(from_name)?;
                        true
                    }
                    Err(e) if is_taken(&to_dir, to_name, &e) => false,
                    Err(_) => return Err(GateError::NoSafeName),
                },
                Err(e) => return Err(e),
            };
        if !moved {
            return Ok(Moved::Taken);
        }
        flush_name(&to_dir, to_name);
        sync_folder(&from_dir);
        Ok(Moved::Moved)
    }

    /// The name, in the folder `folder` (a stored path), of the very file `expect` of `len` bytes,
    /// if it is there under any name (renamed by the person, say). Never follows a link.
    pub fn find_in_folder(
        &self,
        folder: &str,
        expect: FileId,
        len: u64,
    ) -> io::Result<Option<String>> {
        let dir = if folder.is_empty() {
            self.root.try_clone()?
        } else {
            let Some((parent, name)) = self.open_stored_folder(folder)? else {
                return Ok(None);
            };
            if !parent.symlink_metadata(name).is_ok_and(|m| m.is_dir()) {
                return Ok(None);
            }
            parent.open_dir(name)?
        };
        for entry in dir.entries()? {
            let Ok(name) = entry?.file_name().into_string() else {
                continue;
            };
            let candidate = dir
                .symlink_metadata(&name)
                .is_ok_and(|m| m.is_file() && m.len() == len);
            if candidate
                && dir
                    .open(&name)
                    .and_then(|f| identity(&f.into_std()))
                    .is_ok_and(|(id, _)| id == expect)
            {
                return Ok(Some(name));
            }
        }
        Ok(None)
    }

    /// The first stored path for `stored` that nothing uses now: `stored` itself, or a numbered
    /// one such as `name (2).ext` in the same folder. Nothing is created or claimed.
    pub fn free_name_at(&self, stored: &str) -> Result<String, GateError> {
        let parts = stored_parts(stored)?;
        let (name, _) = parts.split_last().ok_or_else(|| invalid("empty path"))?;
        let folder = stored.rsplit_once('/').map_or("", |(f, _)| f);
        let Some((dir, _)) = self.open_stored_folder(stored)? else {
            // The folder is not there yet: every name in it is free.
            return Ok(stored.to_string());
        };
        let (free, _) = self.free_name(&dir, folder, name)?;
        Ok(stored_path(folder, &free))
    }

    /// Removes the folder at the stored path `stored` only if it is empty (never anything in it).
    /// Returns `false` if something is in it; `Ok(false)` too if there is no folder there.
    pub fn remove_empty_folder(&self, stored: &str) -> io::Result<bool> {
        let Some((dir, name)) = self.open_stored_folder(stored)? else {
            return Ok(false);
        };
        match dir.symlink_metadata(name) {
            Ok(meta) if meta.is_dir() => {}
            Ok(_) => return Ok(false),
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(e) => return Err(e),
        }
        let empty = dir.read_dir(name)?.next().is_none();
        if !empty {
            return Ok(false);
        }
        // Removing a folder never removes anything in it: if something arrived meanwhile, this
        // fails and the folder stays.
        match dir.remove_dir(name) {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == io::ErrorKind::DirectoryNotEmpty => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// The full path of the stored file `stored`, for handing it to the system Trash (which takes
    /// paths, not handles), only once that path is proven to lead to exactly that file: the same
    /// file as `expect`, reached without any link or junction on the way. Refused for a place
    /// opened through the admin helper.
    pub fn ambient_path(&self, stored: &str, expect: FileId) -> io::Result<std::path::PathBuf> {
        let root = self.root_path.as_ref().ok_or_else(|| {
            io::Error::other(
                "it is in another person's account, so PCTwin cannot move it to the Trash",
            )
        })?;
        let parts = stored_parts(stored)?;
        let real_root = std::fs::canonicalize(root)?;
        let mut wanted = real_root.clone();
        for part in &parts {
            wanted.push(part);
        }
        let real = std::fs::canonicalize(&wanted)?;
        // Part by part, as the drive compares names: a folder the person already had may be
        // spelled differently on the disk ("Docs") from how it was sent ("docs").
        if !same_spelling(&real, &wanted) {
            return Err(io::Error::other("the path leads somewhere else"));
        }
        if !std::fs::symlink_metadata(&real)?.is_file() {
            return Err(io::Error::other("not a file"));
        }
        let file = std::fs::File::open(&real)?;
        if identity(&file)?.0 != expect {
            return Err(io::Error::other("a different file is there now"));
        }
        Ok(real)
    }

    /// After a restart: the partly received temporary file at the stored path `temp`, opened again
    /// to continue it, for a file sent as `sent` of exactly `announced` bytes. Nothing in it counts
    /// as arrived until the caller checks it ([`IncomingFile::count_arrived`]). Only a regular file
    /// with a PCTwin temporary name no longer than `announced` is accepted; dropping it unfinished
    /// removes it, as for any file being received.
    pub fn reopen_incoming<'d>(
        &'d self,
        sent: &IncomingPath,
        temp: &str,
        announced: u64,
    ) -> Result<IncomingFile<'d>, GateError> {
        let not_temp = || GateError::Io(invalid("not a PCTwin temporary file"));
        let (dir, temp_name) = self.open_stored_folder(temp)?.ok_or_else(not_temp)?;
        let meta = dir.symlink_metadata(temp_name).map_err(|_| not_temp())?;
        if !is_temp_name(temp_name) || !meta.is_file() {
            return Err(not_temp());
        }
        if meta.len() > announced {
            return Err(GateError::OutsideFile);
        }
        let mut options = OpenOptions::new();
        options.read(true).write(true);
        let file = dir.open_with(temp_name, &options)?;
        let host = Platform::host();
        let mut changes: Vec<NameChange> = Vec::new();
        for part in sent.components() {
            for c in convert_name(part, host).changes {
                if !changes.contains(&c) {
                    changes.push(c);
                }
            }
        }
        let name = sent
            .components()
            .last()
            .map(|n| convert_name(n, host).name)
            .ok_or(GateError::Path(PathError::Empty))?;
        Ok(IncomingFile {
            destination: self,
            dir,
            file: Some(file),
            temp_name: Some(temp_name.to_string()),
            announced,
            received: 0,
            cursor: 0,
            arrived: BTreeMap::new(),
            folder: temp.rsplit_once('/').map_or("", |(f, _)| f).to_string(),
            name,
            sent_path: sent.original().to_string(),
            changes,
            modified: None,
            created: Vec::new(),
        })
    }

    /// After a restart: the sealed temporary file at the stored path `temp` (every byte checked
    /// and on disk before the crash), to give the real name a file sent as `sent` gets. Only a
    /// regular file with a PCTwin temporary name is accepted.
    pub fn reopen_sealed<'d>(
        &'d self,
        sent: &IncomingPath,
        temp: &str,
    ) -> Result<Sealed<'d>, GateError> {
        let not_temp = || GateError::Io(invalid("not a PCTwin temporary file"));
        let (dir, temp_name) = self.open_stored_folder(temp)?.ok_or_else(not_temp)?;
        if !is_temp_name(temp_name)
            || !dir
                .symlink_metadata(temp_name)
                .is_ok_and(|meta| meta.is_file())
        {
            return Err(not_temp());
        }
        let host = Platform::host();
        let mut changes: Vec<NameChange> = Vec::new();
        for part in sent.components() {
            for c in convert_name(part, host).changes {
                if !changes.contains(&c) {
                    changes.push(c);
                }
            }
        }
        let name = sent
            .components()
            .last()
            .map(|n| convert_name(n, host).name)
            .ok_or(GateError::Path(PathError::Empty))?;
        Ok(Sealed {
            destination: self,
            dir,
            temp: Some(temp_name.to_string()),
            folder: temp.rsplit_once('/').map_or("", |(f, _)| f).to_string(),
            name,
            sent_path: sent.original().to_string(),
            changes,
        })
    }

    /// The full path of the folder `folder` (a stored path, held as `dir`): its canonical path,
    /// which has no link or junction in it, proven to be that very folder by its identity.
    /// Refused for a place opened through the admin helper (it has no path).
    fn folder_path(&self, dir: &Dir, folder: &str) -> io::Result<std::path::PathBuf> {
        let root = self
            .root_path
            .as_ref()
            .ok_or_else(|| io::Error::other("this place has no path"))?;
        let parts = if folder.is_empty() {
            Vec::new()
        } else {
            stored_parts(folder)?
        };
        let real_root = std::fs::canonicalize(root)?;
        let mut wanted = real_root.clone();
        for part in &parts {
            wanted.push(part);
        }
        let real = std::fs::canonicalize(&wanted)?;
        let held = identity(&dir.try_clone()?.into_std_file())?.0;
        if identity(&open_folder(&real)?)?.0 != held {
            return Err(io::Error::other("a different folder is there now"));
        }
        Ok(real)
    }

    /// Opens the folder of the stored path `stored` without following links or creating
    /// anything: the folder and the file name, or `None` if a folder on the way is missing.
    fn open_stored_folder<'s>(&self, stored: &'s str) -> io::Result<Option<(Dir, &'s str)>> {
        let parts = stored_parts(stored)?;
        let (name, folders) = parts.split_last().ok_or_else(|| invalid("empty path"))?;
        let mut dir = self.root.try_clone()?;
        for folder in folders {
            match dir.symlink_metadata(folder) {
                Ok(meta) if meta.is_dir() => {}
                Ok(_) => return Ok(None),
                Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
                Err(e) => return Err(e),
            }
            dir = dir.open_dir(folder)?;
        }
        Ok(Some((dir, name)))
    }

    /// Where a file sent as `path` would be stored, if a file is already there: its stored path
    /// and size. Names are converted exactly as for writing; nothing is created and no link is
    /// followed out of the approved folder.
    pub fn find(&self, path: &IncomingPath) -> Option<(String, u64)> {
        let host = Platform::host();
        let (file_name, folders) = path.components().split_last()?;
        let mut dir = self.root.try_clone().ok()?;
        let mut shown = Vec::new();
        for folder in folders {
            let name = convert_name(folder, host).name;
            if !dir.symlink_metadata(&name).ok()?.is_dir() {
                return None;
            }
            dir = dir.open_dir(&name).ok()?;
            shown.push(name);
        }
        let name = convert_name(file_name, host).name;
        let meta = dir.symlink_metadata(&name).ok()?;
        if !meta.is_file() {
            return None;
        }
        shown.push(name);
        Some((shown.join("/"), meta.len()))
    }

    /// Opens a stored file to read, by its stored path (as [`find`](Self::find) or
    /// [`Finished::final_path`] give it). Never follows a link.
    pub fn open_read(&self, stored: &str) -> io::Result<std::fs::File> {
        let parts: Vec<&str> = stored.split('/').collect();
        let (file_name, folders) = parts
            .split_last()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "empty path"))?;
        let not_found = || io::Error::new(io::ErrorKind::NotFound, "not a stored file");
        let mut dir = self.root.try_clone()?;
        for folder in folders {
            if !dir.symlink_metadata(folder)?.is_dir() {
                return Err(not_found());
            }
            dir = dir.open_dir(folder)?;
        }
        if !dir.symlink_metadata(file_name)?.is_file() {
            return Err(not_found());
        }
        Ok(dir.open(file_name)?.into_std())
    }

    /// The first name for `name` in `dir` that nothing uses now: `name` itself, or a numbered one
    /// such as `name (2).ext`. Nothing is claimed: another program can still take it first, so a
    /// claim never replaces anything and a taken name is simply tried again.
    fn free_name(&self, dir: &Dir, folder: &str, name: &str) -> Result<(String, bool), GateError> {
        if !name_taken(dir, name)? {
            return Ok((name.to_string(), false));
        }
        let hints = || {
            self.clash_hints
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
        };
        let (stem, ext) = split_extension(name);
        // Numbered names are tried on the stem cut to leave room for the number, so long names
        // that share that start compete for the same numbers: file the hint under the cut stem.
        let base = cut_to(
            stem,
            MAX_COMPONENT_BYTES.saturating_sub(ext.len() + MAX_NUMBER_BYTES),
        );
        let key = hints().key(folder, &format!("{base}{ext}"));
        let start = hints().next.get(&key).copied().unwrap_or(2).max(2);
        for attempt in start..start.saturating_add(MAX_CLASH_ATTEMPTS) {
            let suffix = format!(" ({attempt}){ext}");
            let candidate = format!(
                "{}{suffix}",
                cut_to(stem, MAX_COMPONENT_BYTES.saturating_sub(suffix.len()))
            );
            if !name_taken(dir, &candidate)? {
                hints().remember(key, attempt + 1);
                return Ok((candidate, true));
            }
        }
        Err(GateError::TooManyClashes)
    }
}

/// Whether something already has `name` in `dir`.
fn name_taken(dir: &Dir, name: &str) -> io::Result<bool> {
    match dir.symlink_metadata(name) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e),
    }
}

/// How a finished file got its real name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Linked {
    /// A second name for the same file: the temporary name is still there until it is kept.
    Hard,
    /// Moved onto its name (drives without hard links): the temporary name is gone.
    Moved,
}

/// Where a folder held by a handle is, as a full path, for the one system call that takes paths
/// (Windows' no-replace move). Worked out only when needed.
type FolderPath<'a> = &'a dyn Fn() -> io::Result<std::path::PathBuf>;

/// Gives the finished file `temp` the name `name` if nothing has it, never replacing anything.
/// Returns `None` when the name is taken. Nothing is ever put under `name` but the whole file: a
/// hard link where the drive has them, else a move the drive itself refuses if the name exists.
/// No placeholder is ever made, so no empty file is ever left under a real name.
fn claim(
    dir: &Dir,
    name: &str,
    temp: &str,
    folder_path: FolderPath<'_>,
) -> Result<Option<Linked>, GateError> {
    // A hard link makes the whole file appear under its name at once, or not at all. The
    // temporary name stays until the journal has recorded the real one.
    match dir.hard_link(temp, dir, name) {
        Ok(()) => Ok(Some(Linked::Hard)),
        Err(e) if is_taken(dir, name, &e) => Ok(None),
        // Some drives (FAT, exFAT) have no hard links.
        Err(_) => Ok(rename_no_replace(dir, temp, name, folder_path)?.then_some(Linked::Moved)),
    }
}

/// Moves `temp` onto `name` in `dir` only if nothing has `name` (see [`move_no_replace`]).
fn rename_no_replace(
    dir: &Dir,
    temp: &str,
    name: &str,
    folder_path: FolderPath<'_>,
) -> Result<bool, GateError> {
    move_no_replace(dir, temp, dir, name, folder_path, folder_path)
}

/// Moves `from` in `from_dir` to `to` in `to_dir` (the same drive) only if nothing has `to`; the
/// drive checks and moves in one step, so another file can never be replaced. `Ok(false)` when the
/// name is taken. Refused where the system cannot do this (nothing is moved rather than moved
/// unsafely).
#[cfg(unix)]
fn move_no_replace(
    from_dir: &Dir,
    from: &str,
    to_dir: &Dir,
    to: &str,
    _from_path: FolderPath<'_>,
    _to_path: FolderPath<'_>,
) -> Result<bool, GateError> {
    use rustix::fs::{RenameFlags, renameat_with};
    use rustix::io::Errno;
    // renameat2(RENAME_NOREPLACE) on Linux, renameatx_np(RENAME_EXCL) on macOS.
    match renameat_with(from_dir, from, to_dir, to, RenameFlags::NOREPLACE) {
        Ok(()) => Ok(true),
        Err(Errno::EXIST) => Ok(false),
        Err(Errno::INVAL | Errno::NOSYS | Errno::NOTSUP) => Err(GateError::NoSafeName),
        Err(e) => Err(GateError::Io(e.into())),
    }
}

/// Windows: `MoveFileExW` without `MOVEFILE_REPLACE_EXISTING` (through `tempfile`, which wraps it
/// safely), on the folders' full paths, each proven to be the folder held.
#[cfg(windows)]
fn move_no_replace(
    _from_dir: &Dir,
    from: &str,
    _to_dir: &Dir,
    to: &str,
    from_path: FolderPath<'_>,
    to_path: FolderPath<'_>,
) -> Result<bool, GateError> {
    let from_folder = from_path().map_err(|_| GateError::NoSafeName)?;
    let to_folder = to_path().map_err(|_| GateError::NoSafeName)?;
    let mut source = tempfile::TempPath::try_from_path(from_folder.join(from))?;
    // Never removed by `tempfile`, whatever happens.
    source.disable_cleanup(true);
    match source.persist_noclobber(to_folder.join(to)) {
        Ok(()) => Ok(true),
        Err(e) => {
            let taken = e.error.kind() == io::ErrorKind::AlreadyExists
                || matches!(e.error.raw_os_error(), Some(80 | 183));
            if taken {
                Ok(false)
            } else {
                Err(GateError::Io(e.error))
            }
        }
    }
}

#[cfg(not(any(unix, windows)))]
fn move_no_replace(
    _from_dir: &Dir,
    _from: &str,
    _to_dir: &Dir,
    _to: &str,
    _from_path: FolderPath<'_>,
    _to_path: FolderPath<'_>,
) -> Result<bool, GateError> {
    Err(GateError::NoSafeName)
}

/// How moving a stored file ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Moved {
    /// It is under its new name now.
    Moved,
    /// Something already has the new name: nothing was moved.
    Taken,
    /// The file there is not the one expected (or there is none): nothing was moved.
    NotThatFile,
}

/// Which file or folder this is on its drive: the drive's number and the file's number on it.
/// Two names with the same identity are the same file. A file that is edited keeps its identity;
/// one deleted and made again usually gets a new one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileId {
    pub volume: u64,
    pub index: u64,
}

/// A stored file's size, modified time and identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stat {
    pub len: u64,
    pub modified: Option<std::time::SystemTime>,
    pub id: FileId,
}

#[cfg(unix)]
fn identity(file: &std::fs::File) -> io::Result<(FileId, u64)> {
    use std::os::unix::fs::MetadataExt;
    let meta = file.metadata()?;
    Ok((
        FileId {
            volume: meta.dev(),
            index: meta.ino(),
        },
        meta.nlink(),
    ))
}

#[cfg(windows)]
fn identity(file: &std::fs::File) -> io::Result<(FileId, u64)> {
    let info = winapi_util::file::information(file)?;
    Ok((
        FileId {
            volume: info.volume_serial_number(),
            index: info.file_index(),
        },
        info.number_of_links(),
    ))
}

/// Makes sure a file's new name is on disk before the journal moves on, by flushing the folder
/// that holds it (a name is in the folder's list, not the file): on Unix through the folder's
/// handle; on Windows by opening the folder itself for writing (backup semantics) and flushing
/// it, and the file too. Best effort: some drives cannot do this, and recovery after a crash then
/// treats a lost name honestly (the file is reported failed).
fn flush_name(dir: &Dir, name: &str) {
    #[cfg(unix)]
    {
        let _ = name;
        if let Ok(dir) = dir.try_clone() {
            let _ = dir.into_std_file().sync_all();
        }
    }
    #[cfg(windows)]
    {
        /// Lets Windows open a folder as a handle.
        const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
        let mut options = OpenOptions::new();
        options.write(true);
        if let Ok(file) = dir.open_with(name, &options) {
            let _ = file.sync_all();
        }
        let mut folder = OpenOptions::new();
        folder.write(true);
        cap_std::fs::OpenOptionsExt::custom_flags(&mut folder, FILE_FLAG_BACKUP_SEMANTICS);
        if let Ok(handle) = dir.open_with(".", &folder) {
            let _ = handle.sync_all();
        }
    }
}

/// Flushes a folder's list of names (after a name left it). Best effort, as [`flush_name`].
fn sync_folder(dir: &Dir) {
    #[cfg(unix)]
    if let Ok(dir) = dir.try_clone() {
        let _ = dir.into_std_file().sync_all();
    }
    #[cfg(windows)]
    {
        let mut folder = OpenOptions::new();
        folder.write(true);
        cap_std::fs::OpenOptionsExt::custom_flags(&mut folder, 0x0200_0000);
        if let Ok(handle) = dir.open_with(".", &folder) {
            let _ = handle.sync_all();
        }
    }
}

/// Longest tag for a temporary name, in bytes.
const MAX_TAG_BYTES: usize = 64;

/// The temporary name for a write tagged `tag`: `.pctwin-<tag>.part`.
pub fn temp_name(tag: &str) -> String {
    format!(".pctwin-{tag}.part")
}

fn is_plain_tag(tag: &str) -> bool {
    !tag.is_empty()
        && tag.len() <= MAX_TAG_BYTES
        && tag
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Whether `name` is one of PCTwin's temporary names.
fn is_temp_name(name: &str) -> bool {
    name.strip_prefix(".pctwin-")
        .and_then(|rest| rest.strip_suffix(".part"))
        .is_some_and(is_plain_tag)
}

fn invalid(why: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, why.to_string())
}

/// Whether two full paths name the same place part by part: the way this system's drives compare
/// names (Windows and macOS drives ignore capital letters and accent forms; Linux compares
/// exactly), so a folder whose name on the disk is spelled differently from how it was sent is
/// still the same folder.
fn same_spelling(a: &Path, b: &Path) -> bool {
    let parts = |p: &Path| -> Vec<String> {
        p.components()
            .map(|c| {
                let name = c.as_os_str().to_string_lossy();
                if cfg!(any(windows, target_os = "macos")) {
                    name.nfc().collect::<String>().to_lowercase()
                } else {
                    name.into_owned()
                }
            })
            .collect()
    };
    parts(a) == parts(b)
}

/// Opens a folder by its full path to read its identity (Windows needs backup semantics to open
/// a folder at all).
fn open_folder(path: &Path) -> io::Result<std::fs::File> {
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(0x0200_0000)
            .open(path)
    }
    #[cfg(not(windows))]
    {
        std::fs::File::open(path)
    }
}

/// Characters a stored name never has on this system. On Windows a `:` would name a hidden stream
/// of a file and a `\` is another separator (stored names never have either: they are converted to
/// lookalikes); on Linux and macOS both are ordinary name characters.
#[cfg(windows)]
const NOT_IN_A_NAME: &[char] = &['\\', ':', '\0'];
#[cfg(not(windows))]
const NOT_IN_A_NAME: &[char] = &['\0'];

/// The parts of a stored path (as the gate gave it), refusing anything that could climb out.
fn stored_parts(stored: &str) -> io::Result<Vec<&str>> {
    let parts: Vec<&str> = stored.split('/').collect();
    if parts
        .iter()
        .any(|p| p.is_empty() || *p == "." || *p == ".." || p.contains(NOT_IN_A_NAME))
    {
        return Err(invalid("not a stored path"));
    }
    Ok(parts)
}

/// Whether a failed claim failed because something already has the name. Windows reports a
/// folder in the way as "access denied", so the name itself is checked too.
fn is_taken(dir: &Dir, name: &str, e: &io::Error) -> bool {
    e.kind() == io::ErrorKind::AlreadyExists || dir.symlink_metadata(name).is_ok()
}

/// What kind of place an approved destination is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Approved {
    /// The signed-in person's own folders.
    MyFolders,
    /// A folder shared by everyone on the new laptop.
    SharedFolder,
    /// A drive the person chose.
    ChosenDrive,
    /// A drive for things kept off the new laptop.
    OffloadDrive,
    /// Another person's account, identified by the system's account ID. Opened only through the
    /// admin helper ([`Destinations::approve_through_helper`]).
    AnotherAccount { account_id: String },
}

/// Longest destination label, in bytes.
const MAX_DESTINATION_ID_BYTES: usize = 64;
/// Longest system account ID accepted, in bytes.
const MAX_ACCOUNT_ID_BYTES: usize = 256;

/// The only places on the new laptop that anything is written to. The old laptop names an entry
/// by its label; the label is only looked up here and never used as a path.
#[derive(Default)]
pub struct Destinations {
    entries: HashMap<String, (Approved, Destination)>,
}

impl Destinations {
    pub fn new() -> Self {
        Self::default()
    }

    /// Approves the folder at `path` under the label `id`. Another person's account is refused
    /// here; it needs [`approve_through_helper`](Self::approve_through_helper).
    pub fn approve(&mut self, id: &str, place: Approved, path: &Path) -> Result<(), GateError> {
        if matches!(place, Approved::AnotherAccount { .. }) {
            return Err(GateError::NeedsAdminHelper);
        }
        self.check_new(id)?;
        let destination = Destination::open(path)?;
        self.entries.insert(id.to_string(), (place, destination));
        Ok(())
    }

    /// Approves another person's account folder, opened by the admin helper and handed over as a
    /// directory handle. Only the admin helper connection may call this.
    pub fn approve_through_helper(
        &mut self,
        id: &str,
        account_id: &str,
        root: Dir,
    ) -> Result<(), GateError> {
        if account_id.is_empty()
            || account_id.len() > MAX_ACCOUNT_ID_BYTES
            || account_id.chars().any(char::is_control)
        {
            return Err(GateError::BadDestinationId);
        }
        self.check_new(id)?;
        let place = Approved::AnotherAccount {
            account_id: account_id.to_string(),
        };
        self.entries
            .insert(id.to_string(), (place, Destination::from_dir(root)));
        Ok(())
    }

    /// The approved place labelled `id`, or [`GateError::UnknownDestination`].
    pub fn get(&self, id: &str) -> Result<&Destination, GateError> {
        self.entries
            .get(id)
            .map(|(_, destination)| destination)
            .ok_or(GateError::UnknownDestination)
    }

    /// What kind of place `id` is.
    pub fn place(&self, id: &str) -> Result<&Approved, GateError> {
        self.entries
            .get(id)
            .map(|(place, _)| place)
            .ok_or(GateError::UnknownDestination)
    }

    /// Every approved label, to tell the old laptop where it may send things.
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }

    fn check_new(&self, id: &str) -> Result<(), GateError> {
        let plain = |c: char| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.');
        if id.is_empty()
            || id.len() > MAX_DESTINATION_ID_BYTES
            || !id.chars().all(plain)
            || id.chars().all(|c| c == '.')
        {
            return Err(GateError::BadDestinationId);
        }
        if self.entries.contains_key(id) {
            return Err(GateError::DuplicateDestination);
        }
        Ok(())
    }
}

/// Opens the folder `name` in `parent`, creating it if there is none; says whether it was
/// created here (a folder that was already there is the person's own).
fn open_or_create_folder(parent: &Dir, name: &str) -> Result<(Dir, bool), GateError> {
    let created = match parent.create_dir(name) {
        Ok(()) => true,
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => false,
        Err(e) => return Err(e.into()),
    };
    // cap-std refuses to follow a link out of the approved folder.
    let meta = parent.symlink_metadata(name)?;
    if !meta.is_dir() {
        return Err(GateError::Conflict);
    }
    Ok((parent.open_dir(name)?, created))
}

/// What a space reservation's result means: reserved, not possible on this drive (the copy goes
/// ahead), or not enough space.
fn reserve_outcome(result: io::Result<()>) -> Result<bool, GateError> {
    match result {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == io::ErrorKind::Unsupported => Ok(false),
        // Windows answers a size beyond what the drive can hold with "invalid parameter".
        Err(e)
            if matches!(
                e.kind(),
                io::ErrorKind::StorageFull
                    | io::ErrorKind::FileTooLarge
                    | io::ErrorKind::QuotaExceeded
                    | io::ErrorKind::InvalidInput
            ) =>
        {
            Err(GateError::NoSpace)
        }
        Err(e) => Err(e.into()),
    }
}

/// Creates a temporary `.pctwin-` file in `dir` that no other name uses.
fn create_temp_file(dir: &Dir) -> Result<(File, String), GateError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    for _ in 0..64 {
        let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let name = format!(".pctwin-{}-{n}.part", std::process::id());
        match dir.open_with(&name, &options) {
            Ok(file) => return Ok((file, name)),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.into()),
        }
    }
    Err(GateError::TooManyClashes)
}

/// A part of a file about to be counted as arrived: where its joined run starts, where it ends,
/// the end of the run it joins after it (if any), and its length.
struct NewPart {
    start: u64,
    end: u64,
    joined_end: Option<u64>,
    len: u64,
}

/// Where a file ended up and what was changed on the way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finished {
    /// The path inside the approved folder, with `/` between folders.
    pub final_path: String,
    /// The path as the old laptop sent it.
    pub sent_path: String,
    pub changes: Vec<NameChange>,
}

/// A file being received. Writing more than announced fails; [`finish`](Self::finish) checks the
/// whole file arrived and only then gives it its real name. Dropping it unfinished removes it.
pub struct IncomingFile<'d> {
    destination: &'d Destination,
    dir: Dir,
    /// Closed before renaming or removing: Windows refuses either while the file is open.
    file: Option<File>,
    temp_name: Option<String>,
    announced: u64,
    /// Bytes that have arrived, each counted once (the parts in `arrived` never overlap).
    received: u64,
    /// Where the next in-order write goes.
    cursor: u64,
    /// The parts of the file that have arrived, as start to end, joined when they touch.
    arrived: BTreeMap<u64, u64>,
    folder: String,
    name: String,
    sent_path: String,
    changes: Vec<NameChange>,
    /// The modified time to give the finished file (the original's).
    modified: Option<std::time::SystemTime>,
    /// Folders made for this file (stored paths, outermost first).
    created: Vec<String>,
}

impl<'d> IncomingFile<'d> {
    /// Reserves disk space for the whole announced size now, so a full disk shows at the start
    /// rather than partway through (the approach rclone takes). Returns `false` when this drive
    /// cannot reserve space (some USB and network drives); the file can still be written.
    pub fn reserve(&mut self) -> Result<bool, GateError> {
        if self.announced == 0 {
            return Ok(true);
        }
        // No system stores a file this big; refuse before asking the drive.
        if self.announced > MAX_FILE_BYTES {
            return Err(GateError::NoSpace);
        }
        let file = self.file.take().ok_or(GateError::Conflict)?.into_std();
        // On Windows the reservation lasts while the file is open; it stays open until finish.
        let result = fs4::FileExt::allocate(&file, self.announced);
        self.file = Some(File::from_std(file));
        reserve_outcome(result)
    }

    /// Writes `buf` at `offset` (a section of a file sent over several lanes). Refused if any of
    /// it falls outside the announced size or on bytes that already arrived, so every byte is
    /// written once and counted once. Parts that touch are joined; a write that would leave more
    /// than [`MAX_FILE_PARTS`] separate parts is refused, so scattered tiny writes cannot fill
    /// memory.
    pub fn write_at(&mut self, offset: u64, buf: &[u8]) -> Result<(), GateError> {
        let Some(part) = self.new_part(offset, buf.len() as u64)? else {
            return Ok(());
        };
        let file = self.file.as_mut().ok_or(GateError::Conflict)?;
        file.seek(SeekFrom::Start(offset))?;
        file.write_all(buf)?;
        self.count(part);
        Ok(())
    }

    /// After a restart: counts `len` bytes at `offset`, already in the reopened file and checked
    /// by the caller against their fingerprint, as arrived, without writing them. The same checks
    /// as [`write_at`](Self::write_at): inside the announced size, never counted twice.
    pub fn count_arrived(&mut self, offset: u64, len: u64) -> Result<(), GateError> {
        if let Some(part) = self.new_part(offset, len)? {
            self.count(part);
        }
        Ok(())
    }

    /// Reads back `buf.len()` bytes at `offset` of the file being received (to check what a
    /// reopened file holds).
    pub fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> io::Result<()> {
        let file = self
            .file
            .as_mut()
            .ok_or_else(|| io::Error::other("the file is closed"))?;
        file.seek(SeekFrom::Start(offset))?;
        file.read_exact(buf)
    }

    /// Checks `len` bytes at `offset` are inside the file, not already arrived, and do not
    /// scatter the file into too many parts; `None` for nothing at all.
    fn new_part(&self, offset: u64, len: u64) -> Result<Option<NewPart>, GateError> {
        let end = offset
            .checked_add(len)
            .filter(|end| *end <= self.announced)
            .ok_or(GateError::OutsideFile)?;
        if len == 0 {
            return Ok(None);
        }
        let before = self.arrived.range(..=offset).next_back();
        if before.is_some_and(|(_, e)| *e > offset)
            || self.arrived.range(offset..end).next().is_some()
        {
            return Err(GateError::Overlap);
        }
        let joins_before = before.filter(|(_, e)| **e == offset).map(|(s, _)| *s);
        let joins_after = self.arrived.get(&end).copied();
        if joins_before.is_none() && joins_after.is_none() && self.arrived.len() >= MAX_FILE_PARTS {
            return Err(GateError::TooScattered);
        }
        Ok(Some(NewPart {
            start: joins_before.unwrap_or(offset),
            end,
            joined_end: joins_after,
            len,
        }))
    }

    fn count(&mut self, part: NewPart) {
        if part.joined_end.is_some() {
            self.arrived.remove(&part.end);
        }
        self.arrived
            .insert(part.start, part.joined_end.unwrap_or(part.end));
        self.received += part.len;
    }

    /// Closes it and keeps the partly received file on disk under its temporary name, for the
    /// journal to continue after a restart (dropping it instead removes it).
    pub fn persist(mut self) {
        drop(self.file.take());
        self.temp_name = None;
    }

    /// Bytes that have arrived so far, each counted once.
    pub fn written(&self) -> u64 {
        self.received
    }

    /// Gives the finished file this modified time, so it keeps the original's (and a later check
    /// can tell it is unchanged).
    pub fn keep_modified_time(&mut self, time: std::time::SystemTime) {
        self.modified = Some(time);
    }

    /// Where the temporary file is: its stored path inside the approved folder.
    pub fn temp_path(&self) -> String {
        stored_path(&self.folder, self.temp_name.as_deref().unwrap_or_default())
    }

    /// The folders made for this file because they were not there (stored paths, outermost
    /// first). Folders that were already there are the person's own and are never listed.
    pub fn created_folders(&self) -> &[String] {
        &self.created
    }

    /// Checks every announced byte arrived, makes sure it is on disk, and gives the file its real
    /// name. On any failure the partial file is removed.
    pub fn finish(self) -> Result<Finished, GateError> {
        Ok(self.seal()?.claim()?.keep())
    }

    /// Checks every announced byte arrived and makes sure all of it (and its modified time) is on
    /// disk, still under its temporary name. On any failure the partial file is removed.
    pub fn seal(mut self) -> Result<Sealed<'d>, GateError> {
        // The count of bytes that arrived decides, never the file's length: a reserved file is
        // already full length, with zeros where nothing has arrived yet.
        if self.received != self.announced {
            return Err(GateError::SizeMismatch {
                announced: self.announced,
                received: self.received,
            });
        }
        let file = self.file.take().ok_or(GateError::Conflict)?;
        file.sync_all()?;
        if let Some(time) = self.modified {
            let file = file.into_std();
            file.set_modified(time)?;
            file.sync_all()?;
        } else {
            drop(file);
        }
        let dir = self.dir.try_clone()?;
        let temp = self.temp_name.take().ok_or(GateError::Conflict)?;
        Ok(Sealed {
            destination: self.destination,
            dir,
            temp: Some(temp),
            folder: std::mem::take(&mut self.folder),
            name: std::mem::take(&mut self.name),
            sent_path: std::mem::take(&mut self.sent_path),
            changes: std::mem::take(&mut self.changes),
        })
    }
}

fn stored_path(folder: &str, name: &str) -> String {
    if folder.is_empty() {
        name.to_string()
    } else {
        format!("{folder}/{name}")
    }
}

/// A received file with every byte checked and on disk, still under its temporary name.
/// [`claim`](Self::claim) gives it its real name. Dropping it removes it.
pub struct Sealed<'d> {
    destination: &'d Destination,
    dir: Dir,
    temp: Option<String>,
    folder: String,
    name: String,
    sent_path: String,
    changes: Vec<NameChange>,
}

impl<'d> Sealed<'d> {
    /// Where the temporary file is: its stored path inside the approved folder.
    pub fn temp_path(&self) -> String {
        stored_path(&self.folder, self.temp.as_deref().unwrap_or_default())
    }

    /// The real name (as a stored path) this file would get now: its own name, or a numbered one
    /// if that is taken. Nothing is claimed yet: record it in the journal first, then
    /// [`claim_as`](Self::claim_as).
    pub fn next_name(&self) -> Result<String, GateError> {
        let (name, _) = self
            .destination
            .free_name(&self.dir, &self.folder, &self.name)?;
        Ok(stored_path(&self.folder, &name))
    }

    /// Gives the file the real name `stored` (a name in its own folder), never replacing anything,
    /// and makes sure the name is on disk. If something took that name meanwhile, the file comes
    /// back unchanged (`Err`) to try [`next_name`](Self::next_name) again. Where the drive has hard
    /// links the temporary name stays until [`Claimed::keep`].
    pub fn claim_as(mut self, stored: &str) -> Result<Result<Claimed<'d>, Self>, GateError> {
        let name = self.name_in_folder(stored)?;
        let temp = self.temp.clone().ok_or(GateError::Conflict)?;
        let folder_path = || self.destination.folder_path(&self.dir, &self.folder);
        let Some(linked) = claim(&self.dir, &name, &temp, &folder_path)? else {
            return Ok(Err(self));
        };
        self.temp = None;
        flush_name(&self.dir, &name);
        Ok(Ok(
            self.claimed(name, (linked == Linked::Hard).then_some(temp))
        ))
    }

    /// Which file it is on its drive, to record before it gets its real name: after a crash only
    /// this very file is ever taken as it.
    pub fn identity(&self) -> io::Result<FileId> {
        let temp = self
            .temp
            .as_deref()
            .ok_or_else(|| io::Error::other("no file"))?;
        Ok(identity(&self.dir.open(temp)?.into_std())?.0)
    }

    /// Gives the file a real name that nothing else uses, never replacing another file (without
    /// a journal: the name is not recorded first).
    pub fn claim(self) -> Result<Claimed<'d>, GateError> {
        let mut sealed = self;
        // Each try fails only if another program took the free name in between.
        for _ in 0..MAX_CLAIM_RACES {
            let name = sealed.next_name()?;
            match sealed.claim_as(&name)? {
                Ok(claimed) => return Ok(claimed),
                Err(back) => sealed = back,
            }
        }
        Err(GateError::TooManyClashes)
    }

    /// The file name part of `stored`, which must be in this file's own folder.
    fn name_in_folder(&self, stored: &str) -> Result<String, GateError> {
        let (folder, name) = stored.rsplit_once('/').unwrap_or(("", stored));
        let parts = stored_parts(stored)?;
        if folder != self.folder || parts.is_empty() {
            return Err(GateError::Io(invalid("not a name in this file's folder")));
        }
        Ok(name.to_string())
    }

    fn claimed(&mut self, name: String, temp: Option<String>) -> Claimed<'d> {
        let mut changes = std::mem::take(&mut self.changes);
        if name != self.name && !changes.contains(&NameChange::NameClash) {
            changes.push(NameChange::NameClash);
        }
        Claimed {
            dir: self.dir.try_clone().ok(),
            temp,
            finished: Finished {
                final_path: stored_path(&self.folder, &name),
                sent_path: std::mem::take(&mut self.sent_path),
                changes,
            },
            _destination: std::marker::PhantomData,
        }
    }
}

/// Most times a free name is taken by another program between finding it and claiming it before
/// giving up.
const MAX_CLAIM_RACES: u32 = 64;

impl Drop for Sealed<'_> {
    fn drop(&mut self) {
        // Never given its real name: never leave it behind.
        if let Some(temp) = self.temp.take() {
            let _ = self.dir.remove_file(temp);
        }
    }
}

/// A received file under its real name. [`keep`](Self::keep) removes the temporary name once
/// the journal no longer needs it (if dropped instead, the temporary name stays for the journal's
/// clean-up).
pub struct Claimed<'d> {
    dir: Option<Dir>,
    /// The temporary name, while the file still has it (drives with hard links).
    temp: Option<String>,
    finished: Finished,
    _destination: std::marker::PhantomData<&'d Destination>,
}

impl Claimed<'_> {
    /// Where it landed and what was changed on the way.
    pub fn finished(&self) -> &Finished {
        &self.finished
    }

    /// Removes the temporary name; the file stays under its real name. If another program holds
    /// the temporary name it stays behind (the journal's clean-up removes it later); the file
    /// under its real name is complete either way.
    pub fn keep(self) -> Finished {
        if let (Some(dir), Some(temp)) = (&self.dir, &self.temp) {
            let _ = dir.remove_file(temp);
        }
        self.finished
    }
}

impl Drop for IncomingFile<'_> {
    fn drop(&mut self) {
        // Unfinished or failed: never leave a partial file behind.
        drop(self.file.take());
        if let Some(temp) = self.temp_name.take() {
            let _ = self.dir.remove_file(temp);
        }
    }
}

impl Write for IncomingFile<'_> {
    /// Writes in order, after the previous in-order write, with the same checks as
    /// [`IncomingFile::write_at`].
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self.write_at(self.cursor, buf) {
            Ok(()) => {
                self.cursor += buf.len() as u64;
                Ok(buf.len())
            }
            Err(GateError::Io(e)) => Err(e),
            Err(e) => Err(io::Error::new(io::ErrorKind::InvalidData, e.to_string())),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self.file.as_mut() {
            Some(file) => file.flush(),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    //! The fallback for drives without hard links (FAT, exFAT) cannot be reached through
    //! [`Destination`] on the test machines' disks, so it is checked directly. So is what a space
    //! reservation's error means, since the test machines' disks can all reserve.
    use super::*;

    #[test]
    fn a_drive_that_cannot_reserve_lets_the_copy_go_ahead_and_a_full_one_does_not() {
        use io::ErrorKind as K;
        assert!(matches!(reserve_outcome(Ok(())), Ok(true)));
        assert!(matches!(
            reserve_outcome(Err(io::Error::from(K::Unsupported))),
            Ok(false)
        ));
        for full in [
            K::StorageFull,
            K::FileTooLarge,
            K::QuotaExceeded,
            K::InvalidInput,
        ] {
            assert!(matches!(
                reserve_outcome(Err(io::Error::from(full))),
                Err(GateError::NoSpace)
            ));
        }
        assert!(matches!(
            reserve_outcome(Err(io::Error::from(K::PermissionDenied))),
            Err(GateError::Io(_))
        ));
    }

    fn folder() -> (tempfile::TempDir, Dir) {
        let root = tempfile::tempdir().unwrap();
        let dir = Dir::open_ambient_dir(root.path(), ambient_authority()).unwrap();
        (root, dir)
    }

    fn path_of(root: &tempfile::TempDir) -> impl Fn() -> io::Result<std::path::PathBuf> + '_ {
        move || std::fs::canonicalize(root.path())
    }

    #[test]
    fn without_hard_links_the_whole_file_is_moved_onto_its_name() {
        let (root, dir) = folder();
        std::fs::write(root.path().join("t.part"), b"abc").unwrap();
        assert!(rename_no_replace(&dir, "t.part", "photo.jpg", &path_of(&root)).unwrap());
        assert_eq!(
            std::fs::read(root.path().join("photo.jpg")).unwrap(),
            b"abc"
        );
        assert!(!root.path().join("t.part").exists());
    }

    #[test]
    fn a_failed_move_leaves_nothing_under_the_real_name() {
        let (root, dir) = folder();
        // The temporary file vanished (another program deleted it).
        assert!(rename_no_replace(&dir, "gone.part", "doc.pdf", &path_of(&root)).is_err());
        assert!(!root.path().join("doc.pdf").exists());
    }

    #[test]
    fn a_taken_name_is_reported_as_taken_and_everything_is_left_alone() {
        let (root, dir) = folder();
        std::fs::write(root.path().join("t.part"), b"new").unwrap();
        std::fs::write(root.path().join("notes.txt"), b"mine").unwrap();
        // Even an empty file is someone's: never replaced.
        std::fs::write(root.path().join("empty.txt"), b"").unwrap();
        std::fs::create_dir(root.path().join("Notes")).unwrap();
        for taken in ["notes.txt", "empty.txt", "Notes"] {
            assert!(
                !rename_no_replace(&dir, "t.part", taken, &path_of(&root)).unwrap(),
                "{taken}"
            );
        }
        assert_eq!(
            std::fs::read(root.path().join("notes.txt")).unwrap(),
            b"mine"
        );
        assert_eq!(std::fs::read(root.path().join("empty.txt")).unwrap(), b"");
        assert!(root.path().join("Notes").is_dir());
        assert_eq!(std::fs::read(root.path().join("t.part")).unwrap(), b"new");
    }

    #[cfg(windows)]
    #[test]
    fn without_a_proven_folder_path_nothing_is_moved() {
        let (root, dir) = folder();
        std::fs::write(root.path().join("t.part"), b"abc").unwrap();
        let no_path = || -> io::Result<std::path::PathBuf> { Err(io::Error::other("none")) };
        assert!(matches!(
            rename_no_replace(&dir, "t.part", "a.txt", &no_path),
            Err(GateError::NoSafeName)
        ));
        assert!(root.path().join("t.part").exists());
        assert!(!root.path().join("a.txt").exists());
    }

    #[test]
    fn a_folder_s_full_path_is_given_only_for_the_very_folder_held() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("a/b")).unwrap();
        std::fs::create_dir(root.path().join("c")).unwrap();
        let dest = Destination::open(root.path()).unwrap();
        let b = dest.root.open_dir("a").unwrap().open_dir("b").unwrap();
        let path = dest.folder_path(&b, "a/b").unwrap();
        assert_eq!(
            path,
            std::fs::canonicalize(root.path().join("a/b")).unwrap()
        );
        assert_eq!(
            dest.folder_path(&dest.root, "").unwrap(),
            std::fs::canonicalize(root.path()).unwrap()
        );
        // Another folder than the one held, or a path that climbs: refused.
        assert!(dest.folder_path(&b, "c").is_err());
        assert!(dest.folder_path(&b, "a/../a/b").is_err());
        // A place with no path (opened through the admin helper): refused.
        let held = Destination::from_dir(dest.root.try_clone().unwrap());
        assert!(held.folder_path(&b, "a/b").is_err());
    }

    #[test]
    fn paths_are_compared_part_by_part_as_the_drive_compares_names() {
        use std::path::Path;
        assert!(same_spelling(Path::new("/a/b/c"), Path::new("/a/b/c")));
        assert!(!same_spelling(Path::new("/a/b/c"), Path::new("/a/bc")));
        assert!(!same_spelling(Path::new("/a/b"), Path::new("/a/b/c")));
        // Composed and decomposed accents are the same name.
        assert!(same_spelling(
            Path::new("/a/Caf\u{e9}"),
            Path::new("/a/Cafe\u{301}")
        ));
        let case = same_spelling(Path::new("/a/Docs"), Path::new("/a/docs"));
        assert_eq!(case, cfg!(any(windows, target_os = "macos")));
    }
}
