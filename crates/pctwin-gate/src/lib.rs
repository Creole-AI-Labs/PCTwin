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
use std::io::{self, Seek, SeekFrom, Write};
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
    #[error("writing failed: {0}")]
    Io(#[from] io::Error),
}

/// An approved folder on the new laptop. Everything written goes inside it.
pub struct Destination {
    root: Dir,
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
        Ok(Self::from_dir(Dir::open_ambient_dir(
            path,
            ambient_authority(),
        )?))
    }

    fn from_dir(root: Dir) -> Self {
        Self {
            root,
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
        let host = Platform::host();
        let (file_name, folders) = path
            .components()
            .split_last()
            .ok_or(GateError::Path(PathError::Empty))?;
        let mut changes = Vec::new();
        let mut shown = Vec::new();
        let mut dir = self.root.try_clone()?;
        for folder in folders {
            let converted = convert_name(folder, host);
            changes.extend(converted.changes.iter().copied());
            dir = open_or_create_folder(&dir, &converted.name)?;
            shown.push(converted.name);
        }
        let converted = convert_name(file_name, host);
        changes.extend(converted.changes.iter().copied());
        let (file, temp_name) = create_temp_file(&dir)?;
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
        })
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

    /// Gives the finished temporary file a real name that nothing else uses, never replacing
    /// another file. Returns the name used.
    fn claim_name(
        &self,
        dir: &Dir,
        folder: &str,
        name: &str,
        temp: &str,
    ) -> Result<(String, bool), GateError> {
        if claim(dir, name, temp)? {
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
            if claim(dir, &candidate, temp)? {
                hints().remember(key, attempt + 1);
                return Ok((candidate, true));
            }
        }
        Err(GateError::TooManyClashes)
    }
}

/// Gives the finished file `temp` the name `name` if nothing has it, never replacing anything.
/// Returns `false` when the name is taken. On failure nothing is left under `name`.
fn claim(dir: &Dir, name: &str, temp: &str) -> Result<bool, GateError> {
    // A hard link makes the whole file appear under its name at once, or not at all.
    match dir.hard_link(temp, dir, name) {
        Ok(()) => {
            // If another program holds the temporary name, it stays behind as a copy; the file
            // under its real name is complete either way.
            let _ = dir.remove_file(temp);
            Ok(true)
        }
        Err(e) if is_taken(dir, name, &e) => Ok(false),
        // Some drives (FAT, exFAT) have no hard links.
        Err(_) => claim_by_reservation(dir, name, temp),
    }
}

/// For drives without hard links: reserve the name (never replacing anything), then move the
/// finished file onto the reservation. On failure the reservation is removed.
fn claim_by_reservation(dir: &Dir, name: &str, temp: &str) -> Result<bool, GateError> {
    let mut reserve = OpenOptions::new();
    reserve.write(true).create_new(true);
    match dir.open_with(name, &reserve) {
        Ok(placeholder) => {
            drop(placeholder);
            match dir.rename(temp, dir, name) {
                Ok(()) => Ok(true),
                Err(e) => {
                    // Never leave the empty reservation under the real name.
                    let _ = dir.remove_file(name);
                    Err(e.into())
                }
            }
        }
        Err(e) if is_taken(dir, name, &e) => Ok(false),
        Err(e) => Err(e.into()),
    }
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

fn open_or_create_folder(parent: &Dir, name: &str) -> Result<Dir, GateError> {
    match parent.create_dir(name) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e.into()),
    }
    // cap-std refuses to follow a link out of the approved folder.
    let meta = parent.symlink_metadata(name)?;
    if !meta.is_dir() {
        return Err(GateError::Conflict);
    }
    Ok(parent.open_dir(name)?)
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
}

impl IncomingFile<'_> {
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
        let end = offset
            .checked_add(buf.len() as u64)
            .filter(|end| *end <= self.announced)
            .ok_or(GateError::OutsideFile)?;
        if buf.is_empty() {
            return Ok(());
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
        let file = self.file.as_mut().ok_or(GateError::Conflict)?;
        file.seek(SeekFrom::Start(offset))?;
        file.write_all(buf)?;
        if joins_after.is_some() {
            self.arrived.remove(&end);
        }
        self.arrived
            .insert(joins_before.unwrap_or(offset), joins_after.unwrap_or(end));
        self.received += buf.len() as u64;
        Ok(())
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

    /// Checks every announced byte arrived, makes sure it is on disk, and gives the file its real
    /// name. On any failure the partial file is removed.
    pub fn finish(mut self) -> Result<Finished, GateError> {
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
        let temp = self.temp_name.clone().ok_or(GateError::Conflict)?;
        let (name, clashed) =
            self.destination
                .claim_name(&self.dir, &self.folder, &self.name, &temp)?;
        // The file now has its real name; nothing is left to clean up.
        self.temp_name = None;
        if clashed {
            self.changes.push(NameChange::NameClash);
        }
        let final_path = if self.folder.is_empty() {
            name
        } else {
            format!("{}/{name}", self.folder)
        };
        Ok(Finished {
            final_path,
            sent_path: std::mem::take(&mut self.sent_path),
            changes: std::mem::take(&mut self.changes),
        })
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

    #[test]
    fn the_reservation_moves_the_finished_file_onto_its_name() {
        let (root, dir) = folder();
        std::fs::write(root.path().join("t.part"), b"abc").unwrap();
        assert!(claim_by_reservation(&dir, "photo.jpg", "t.part").unwrap());
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
        assert!(claim_by_reservation(&dir, "doc.pdf", "gone.part").is_err());
        assert!(!root.path().join("doc.pdf").exists());
    }

    #[test]
    fn a_taken_name_is_reported_as_taken_and_left_alone() {
        let (root, dir) = folder();
        std::fs::write(root.path().join("t.part"), b"new").unwrap();
        std::fs::write(root.path().join("notes.txt"), b"mine").unwrap();
        std::fs::create_dir(root.path().join("Notes")).unwrap();
        assert!(!claim_by_reservation(&dir, "notes.txt", "t.part").unwrap());
        assert!(!claim_by_reservation(&dir, "Notes", "t.part").unwrap());
        assert_eq!(
            std::fs::read(root.path().join("notes.txt")).unwrap(),
            b"mine"
        );
        assert!(root.path().join("Notes").is_dir());
        assert_eq!(std::fs::read(root.path().join("t.part")).unwrap(), b"new");
    }
}
