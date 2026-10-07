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
//!   a link cannot redirect a write elsewhere. A file is written under a hidden temporary name and
//!   gets its real name only once every announced byte has arrived; an unfinished file is removed.
//!   Nothing is overwritten: a taken name becomes `name (2).ext`, found in constant time even when
//!   thousands of files share a name.
//!
//! Received files are data: nothing here runs, opens or interprets their contents.

use std::collections::HashMap;
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};

use cap_std::ambient_authority;
use cap_std::fs::{Dir, File, OpenOptions};
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
    if platform == Platform::Windows {
        let replaced: String = name.chars().map(windows_safe_char).collect();
        if replaced != name {
            note(NameChange::ForbiddenCharacters, &mut changes);
            name = replaced;
        }
        if looks_like_short_name(&name) {
            note(NameChange::ShortNameAlias, &mut changes);
            name = mark_after_stem(&name);
        }
    }
    // Each step can undo another (shortening can expose a trailing space, renaming can lengthen),
    // so repeat until nothing changes. Every step only shrinks or marks once, so this ends fast.
    for _ in 0..8 {
        let before = name.clone();
        if name.len() > MAX_COMPONENT_BYTES {
            name = shorten(&name, MAX_COMPONENT_BYTES);
            note(NameChange::Shortened, &mut changes);
        }
        if platform == Platform::Windows {
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

/// Adds `_` right after the stem: `CON.txt` → `CON_.txt`.
fn mark_after_stem(name: &str) -> String {
    let (stem, rest) = split_stem(name);
    format!("{stem}_{rest}")
}

/// `CON`, `PRN`, `AUX`, `NUL`, `CONIN$`, `CONOUT$`, `COM0`–`COM9`, `LPT0`–`LPT9` (and the
/// superscript-digit forms), whatever the case and extension. Windows ignores trailing spaces in
/// the stem when it checks.
fn is_windows_reserved(name: &str) -> bool {
    let stem = split_stem(name).0.trim_end_matches(' ').to_uppercase();
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
/// three. Windows may resolve such a name to an existing long-named file or folder.
fn looks_like_short_name(name: &str) -> bool {
    let (stem, rest) = split_stem(name);
    let ext = rest.strip_prefix('.').unwrap_or(rest);
    let Some((base, digits)) = stem.rsplit_once('~') else {
        return false;
    };
    (1..=6).contains(&base.chars().count())
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
    #[error("writing failed: {0}")]
    Io(#[from] io::Error),
}

/// An approved folder on the new laptop. Everything written goes inside it.
pub struct Destination {
    root: Dir,
    /// The next number to try for each name in each folder, so clashes are found in constant time.
    next_number: Mutex<HashMap<String, u32>>,
}

/// Distinguishes temporary names created by this process.
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

impl Destination {
    /// Opens the approved folder at `path`. This is the only place an ordinary path is used.
    pub fn open(path: &Path) -> io::Result<Self> {
        Ok(Self {
            root: Dir::open_ambient_dir(path, ambient_authority())?,
            next_number: Mutex::new(HashMap::new()),
        })
    }

    /// Starts receiving a file at `path` (inside the approved folder) that must be exactly
    /// `announced` bytes. It is written under a hidden temporary name; [`IncomingFile::finish`]
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
            folder: shown.join("/"),
            name: converted.name,
            sent_path: path.original().to_string(),
            changes,
        })
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
        let key = format!("{folder}\u{0}{}", name.to_lowercase());
        let mut next = self
            .next_number
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let start = next.get(&key).copied().unwrap_or(1);
        let (stem, ext) = split_extension(name);
        let mut reserve = OpenOptions::new();
        reserve.write(true).create_new(true);
        for attempt in start..start.saturating_add(MAX_CLASH_ATTEMPTS) {
            let candidate = if attempt == 1 {
                name.to_string()
            } else {
                let suffix = format!(" ({attempt}){ext}");
                format!(
                    "{}{suffix}",
                    cut_to(stem, MAX_COMPONENT_BYTES.saturating_sub(suffix.len()))
                )
            };
            // Reserve the name first (this never replaces anything), then move the finished
            // file onto the reservation.
            match dir.open_with(&candidate, &reserve) {
                Ok(placeholder) => {
                    drop(placeholder);
                    next.insert(key, attempt + 1);
                    dir.rename(temp, dir, &candidate)?;
                    return Ok((candidate, attempt > 1));
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e.into()),
            }
        }
        Err(GateError::TooManyClashes)
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

/// Creates a hidden temporary file in `dir` that no other name uses.
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
    received: u64,
    folder: String,
    name: String,
    sent_path: String,
    changes: Vec<NameChange>,
}

impl IncomingFile<'_> {
    /// Checks every announced byte arrived, makes sure it is on disk, and gives the file its real
    /// name. On any failure the partial file is removed.
    pub fn finish(mut self) -> Result<Finished, GateError> {
        if self.received != self.announced {
            return Err(GateError::SizeMismatch {
                announced: self.announced,
                received: self.received,
            });
        }
        let file = self.file.take().ok_or(GateError::Conflict)?;
        file.sync_all()?;
        drop(file);
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
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let room = self.announced - self.received;
        if buf.len() as u64 > room {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "more data than the old laptop announced",
            ));
        }
        let file = self
            .file
            .as_mut()
            .ok_or_else(|| io::Error::other("the file is already closed"))?;
        let n = file.write(buf)?;
        self.received += n as u64;
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        match self.file.as_mut() {
            Some(file) => file.flush(),
            None => Ok(()),
        }
    }
}
