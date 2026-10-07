//! PCTwin safety gate (Security Design Part B): the new laptop trusts nothing that arrives.
//!
//! - [`IncomingPath::parse`] checks every path the old laptop sends. Paths are relative to an
//!   approved folder, use `/` between folders, and anything that could climb out (`..`, an
//!   absolute path, empty or `.` parts, NUL) is refused. Sizes and depth are capped. Names are
//!   stored in one Unicode form (NFC), so the same accented name never appears twice.
//! - [`convert_name`] makes a name storable on the target system (Engineering Plan 9.6): on
//!   Windows, forbidden characters become lookalikes, reserved names such as `CON` are renamed and
//!   trailing dots and spaces are trimmed. Every change is reported as a [`NameChange`].
//! - [`Destination`] writes only inside an approved folder, through a cap-std directory handle, so
//!   a link or a race cannot redirect a write elsewhere. It never overwrites (a taken name becomes
//!   `name (2).ext`) and each file must receive exactly the number of bytes announced.
//!
//! Received files are data: nothing here runs, opens or interprets their contents.

use std::io::{self, Write};
use std::path::Path;

use cap_std::ambient_authority;
use cap_std::fs::{Dir, File, OpenOptions};
use unicode_normalization::UnicodeNormalization;

/// Longest single file or folder name, in bytes (the limit on every supported system).
pub const MAX_COMPONENT_BYTES: usize = 255;
/// Deepest folder nesting accepted.
pub const MAX_DEPTH: usize = 128;
/// Longest whole path accepted, in bytes.
pub const MAX_PATH_BYTES: usize = 4096;
/// Most alternative names tried before giving up on a clash.
const MAX_CLASH_ATTEMPTS: u32 = 9_999;

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
pub struct IncomingPath(Vec<String>);

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
        Ok(Self(components))
    }

    /// The folder and file names, outermost first.
    pub fn components(&self) -> &[String] {
        &self.0
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
    if platform == Platform::Windows {
        let replaced: String = name.chars().map(windows_safe_char).collect();
        if replaced != name {
            changes.push(NameChange::ForbiddenCharacters);
            name = replaced;
        }
        let trimmed = name.trim_end_matches(['.', ' ']);
        if trimmed.len() != name.len() {
            changes.push(NameChange::TrailingDotsOrSpaces);
            name = if trimmed.is_empty() {
                "_".to_string()
            } else {
                trimmed.to_string()
            };
        }
        if is_windows_reserved(&name) {
            changes.push(NameChange::ReservedName);
            let stem_end = name.find('.').unwrap_or(name.len());
            name.insert(stem_end, '_');
        }
    }
    if name.len() > MAX_COMPONENT_BYTES {
        name = shorten(&name, MAX_COMPONENT_BYTES);
        changes.push(NameChange::Shortened);
    }
    if name.is_empty() {
        name = "_".to_string();
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

/// `CON`, `PRN`, `AUX`, `NUL`, `CONIN$`, `CONOUT$`, `COM1`–`COM9`, `LPT1`–`LPT9` (and the
/// superscript-digit forms), whatever the case and extension.
fn is_windows_reserved(name: &str) -> bool {
    let stem = name
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end()
        .to_uppercase();
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
        && matches!(rest[0], '1'..='9' | '\u{B9}' | '\u{B2}' | '\u{B3}')
}

/// Shortens `name` to at most `max` bytes, keeping its extension and whole characters.
fn shorten(name: &str, max: usize) -> String {
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 && name.len() - i <= 16 => (&name[..i], &name[i..]),
        _ => (name, ""),
    };
    let budget = max.saturating_sub(ext.len());
    let mut cut = budget.min(stem.len());
    while !stem.is_char_boundary(cut) {
        cut -= 1;
    }
    format!("{}{ext}", &stem[..cut])
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
}

impl Destination {
    /// Opens the approved folder at `path`. This is the only place an ordinary path is used.
    pub fn open(path: &Path) -> io::Result<Self> {
        Ok(Self {
            root: Dir::open_ambient_dir(path, ambient_authority())?,
        })
    }

    /// Creates a new file at `path` (inside the approved folder) that must receive exactly
    /// `announced` bytes. Names are made storable on this system; a taken name gets a number.
    pub fn create_file(
        &self,
        path: &IncomingPath,
        announced: u64,
    ) -> Result<IncomingFile, GateError> {
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
        let (file, name) = create_new_file(&dir, &converted.name, &mut changes)?;
        shown.push(name);
        Ok(IncomingFile {
            file,
            announced,
            received: 0,
            finished: Finished {
                final_path: shown.join("/"),
                changes,
            },
        })
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

/// Creates `name` without ever replacing anything; a taken name becomes `stem (2).ext` and so on.
fn create_new_file(
    dir: &Dir,
    name: &str,
    changes: &mut Vec<NameChange>,
) -> Result<(File, String), GateError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name, ""),
    };
    for attempt in 1..=MAX_CLASH_ATTEMPTS {
        let candidate = if attempt == 1 {
            name.to_string()
        } else {
            let suffix = format!(" ({attempt}){ext}");
            shorten_stem(stem, MAX_COMPONENT_BYTES.saturating_sub(suffix.len())) + &suffix
        };
        match dir.open_with(&candidate, &options) {
            Ok(file) => {
                if attempt > 1 {
                    changes.push(NameChange::NameClash);
                }
                return Ok((file, candidate));
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.into()),
        }
    }
    Err(GateError::TooManyClashes)
}

fn shorten_stem(stem: &str, max: usize) -> String {
    let mut cut = max.min(stem.len());
    while !stem.is_char_boundary(cut) {
        cut -= 1;
    }
    stem[..cut].to_string()
}

/// Where a file ended up and what was changed on the way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finished {
    /// The path inside the approved folder, with `/` between folders.
    pub final_path: String,
    pub changes: Vec<NameChange>,
}

/// A file being received. Writing more than announced fails; [`finish`](Self::finish) checks the
/// whole file arrived.
pub struct IncomingFile {
    file: File,
    announced: u64,
    received: u64,
    finished: Finished,
}

impl IncomingFile {
    /// Checks every announced byte arrived and makes sure it is on disk.
    pub fn finish(mut self) -> Result<Finished, GateError> {
        if self.received != self.announced {
            return Err(GateError::SizeMismatch {
                announced: self.announced,
                received: self.received,
            });
        }
        self.file.flush()?;
        self.file.sync_all()?;
        Ok(self.finished)
    }
}

impl Write for IncomingFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let room = self.announced - self.received;
        if buf.len() as u64 > room {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "more data than the old laptop announced",
            ));
        }
        let n = self.file.write(buf)?;
        self.received += n as u64;
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}
