//! PCTwin safety gate (security design notes): the new laptop trusts nothing that arrives.
//!
//! - [`IncomingPath::parse`] checks every path the old laptop sends. Paths are relative to an
//!   approved folder, use `/` between folders, and anything that could climb out (`..`, an
//!   absolute path, empty or `.` parts, NUL) is refused. Sizes and depth are capped. Names are
//!   held in one Unicode form (NFC), so the same accented name never appears twice.
//! - [`convert_name`] makes a name storable on the target system (engineering notes). On
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

mod birth_hold;
mod drive;
mod fill;
#[cfg(target_os = "macos")]
mod macos_check;
mod marks;
mod naming;
mod original;
mod read_back;
mod siblings;
#[cfg(unix)]
mod unix_undo;

pub use drive::{DriveKind, DriveProfile, FAT_LARGEST_FILE};
pub use fill::{Fill, FillFinish, FillPlan, FillRemoval, FillSteps, FillTag, Proof};
pub use marks::download_mark_of;
pub use naming::{DriveCheck, Naming};

/// For tests: what the first and second probe files of a drive test hold, so a test can leave
/// probe files as a crash would.
#[doc(hidden)]
pub fn probe_contents() -> (&'static [u8], &'static [u8]) {
    (naming::PROBE_A, naming::PROBE_B)
}
pub use original::open_original;
pub use read_back::{CheckedFrom, ReadBack, Verification, WhyMemory};
pub use siblings::{MOST_TRIES, NAME_BYTES, sibling_name};
#[cfg(unix)]
pub use unix_undo::{Check, Checked, Context, Resolution, Step, private_name};

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

/// Which names a drive takes. The drive decides, not the system ([`DriveProfile`]); a
/// [`Platform`] still names its own drives' rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameRules {
    /// Windows' rules: no `< > : " / \ | ? *` or control characters, no trailing dot or space, no
    /// reserved names (`CON`) or short-name lookalikes (`LONGFO~1`). FAT, exFAT, NTFS, network
    /// and unknown drives, on every system.
    Windows,
    /// Any name the system allows: known POSIX drives on Linux and macOS.
    Posix,
}

impl From<Platform> for NameRules {
    fn from(platform: Platform) -> Self {
        match platform {
            Platform::Windows => Self::Windows,
            Platform::MacOs | Platform::Linux => Self::Posix,
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

/// Makes `name` storable under `rules` (a drive's [`DriveProfile::name_rules`], or a
/// [`Platform`]'s own). The name is stored composed (NFC); other changes are reported.
pub fn convert_name(name: &str, rules: impl Into<NameRules>) -> Converted {
    let mut name: String = name.nfc().collect();
    let mut changes = Vec::new();
    let note = |c: NameChange, changes: &mut Vec<NameChange>| {
        if !changes.contains(&c) {
            changes.push(c);
        }
    };
    let windows = rules.into() == NameRules::Windows;
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
    /// The drive refused the name itself (its own rules), on a drive whose way of naming files
    /// is known to work: never reported as an unsafe drive.
    #[error("this drive won't accept this file's name")]
    NameRejected,
    /// While a file was being filled in under its real name, another program put its own file
    /// there: that file is kept, and so is PCTwin's checked copy.
    #[error(
        "another program put its own file at this name while PCTwin was copying; that file was kept"
    )]
    ReplacedWhileFilling,
    /// The file is bigger than this drive holds in one file (FAT32: 4 GB less one byte).
    #[error(
        "this file is too big for this drive, which holds files of up to {} GB each",
        (largest + 1) >> 30
    )]
    TooBigForDrive { largest: u64 },
    #[error("the file was not read back and checked before getting its name")]
    NotReadBack,
    #[error("the read-back was given the wrong list of block fingerprints")]
    BadBlockList,
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
    /// How many names were checked while choosing free names (see [`name_checks`](Self::name_checks)).
    name_checks: std::sync::atomic::AtomicU64,
    /// The drive it is on, read when it was opened: its name rules and limits.
    profile: DriveProfile,
    /// A profile a test forced on it, for every drive under it.
    forced: bool,
    /// Each drive under it, by its number, judged once: a drive mounted inside the approved
    /// folder gets its own rules and limits, not the folder's.
    profiles: Mutex<HashMap<u64, DriveProfile>>,
    /// Whether files can be read back past the cache, tested once per drive (by its number):
    /// an approved folder can hold another drive mounted inside it.
    read_back: Mutex<HashMap<u64, CheckedFrom>>,
    /// What testing each drive under it found, by the drive's number: tested once, before its
    /// first file is named (a folder under it can be another drive).
    checks: Mutex<HashMap<u64, DriveCheck>>,
    /// For tests: how a fill copies (see [`with_fill_hook`](Self::with_fill_hook)).
    fill_hook: Option<fill::FillHook>,
    /// For tests: the faults this place fakes.
    #[cfg(feature = "fault-plan")]
    faults: Faults,
}

/// For tests (feature `fault-plan`): faults a destination fakes, to prove the read-back's guards.
#[cfg(feature = "fault-plan")]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FaultPlan {
    /// A byte stored or read back wrong.
    pub flip: Option<Flip>,
    /// The drive refuses to be read past the cache (some network drives and USB sticks).
    pub uncached_refused: bool,
    /// The drive takes reads past the cache, but each file is read through memory anyway (as
    /// compressed or encrypted files are).
    pub served_from_memory: bool,
    /// Each file is read from the drive at first, then another program compresses it while it
    /// is read, so the rest comes through memory.
    pub stored_specially_while_read: bool,
}

/// For tests: when a byte goes wrong.
#[cfg(feature = "fault-plan")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flip {
    /// The first byte of the first write on this destination is stored flipped.
    OnWriteOnce,
    /// The first byte of every write is stored flipped.
    OnEveryWrite,
    /// The byte at `at` in each file is stored right but reads back flipped.
    OnRead { at: u64 },
}

#[cfg(feature = "fault-plan")]
#[derive(Default)]
struct Faults {
    plan: FaultPlan,
    flipped_once: std::sync::atomic::AtomicBool,
}

/// Clash hints keyed by a hash of the folder and name, folded so every spelling a disk might treat
/// as the same lands on one key. Entries exist only for names that clashed, and are capped.
#[derive(Default)]
struct ClashHints {
    hasher: RandomState,
    next: HashMap<u64, u32>,
}

impl ClashHints {
    /// The key for `name` in `folder`. Folded to one case unless the drive tells case apart
    /// (then `A.jpg` and `a.jpg` are different names, and numbering one never skips numbers of
    /// the other).
    fn key(&self, folder: &str, name: &str, case_apart: bool) -> u64 {
        let fold = |s: &str| -> String {
            if case_apart {
                return s.to_string();
            }
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

    /// How many names have been checked so far while choosing free names for files written here.
    /// A few per file is normal; a count that grows with the number of files already there means
    /// numbering has become slow (the gate's tests assert it, rather than timing it).
    pub fn name_checks(&self) -> u64 {
        self.name_checks.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// For tests: behave as on a drive with `profile` (a FAT drive, say, which no test machine
    /// has), whatever drive the folder is really on. Everything that depends on the drive
    /// follows it: name rules, the largest file, download markers.
    #[doc(hidden)]
    #[must_use]
    pub fn with_profile(mut self, profile: DriveProfile) -> Self {
        self.profile = profile;
        self.forced = true;
        self
    }

    /// The profile of the drive the folder `dir` (under this place) is on, judged once per
    /// drive. A drive whose number cannot be read takes the approved folder's own.
    fn profile_of(&self, dir: &Dir) -> DriveProfile {
        if self.forced {
            return self.profile;
        }
        let Ok(key) = drive::volume_key(dir) else {
            return self.profile;
        };
        *self
            .profiles
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .entry(key)
            .or_insert_with(|| DriveProfile::read(dir))
    }

    /// The folders of a path that are already there, opened in turn without following a link
    /// or making anything: the deepest one reached, and the stored names of all the folders (each
    /// converted for the drive it is made on; folders not there yet will be made on the deepest
    /// one's drive).
    fn walk_folders(&self, folders: &[String]) -> io::Result<(Dir, Vec<String>)> {
        let mut dir = self.root.try_clone()?;
        let mut names = Vec::new();
        let mut there = true;
        for folder in folders {
            let name = convert_name(folder, self.profile_of(&dir).name_rules()).name;
            if there {
                match dir.symlink_metadata(&name) {
                    Ok(meta) if meta.is_dir() => dir = dir.open_dir(&name)?,
                    _ => there = false,
                }
            }
            names.push(name);
        }
        Ok((dir, names))
    }

    /// What is known about the drive this place is on, read when it was opened.
    pub fn profile(&self) -> DriveProfile {
        self.profile
    }

    /// The largest single file this place's drive holds, if it has a limit (FAT32: 4 GB less
    /// one byte), so the plan can refuse a file that is too big before the move starts.
    pub fn largest_file(&self) -> Option<u64> {
        self.profile.largest_file()
    }

    /// Another handle on the very same approved folder (for a worker that finishes part of an
    /// undo with a deadline, which must own what it works on).
    pub fn try_clone(&self) -> io::Result<Self> {
        Ok(Self {
            root: self.root.try_clone()?,
            root_path: self.root_path.clone(),
            clash_hints: Mutex::new(ClashHints::default()),
            name_checks: Default::default(),
            profile: self.profile,
            forced: self.forced,
            profiles: Mutex::new(
                self.profiles
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .clone(),
            ),
            read_back: Mutex::new(self.read_back_known()),
            #[cfg(feature = "fault-plan")]
            faults: Faults {
                plan: self.faults.plan,
                flipped_once: Default::default(),
            },
            checks: Mutex::new(
                self.checks
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .clone(),
            ),
            fill_hook: self.fill_hook.clone(),
        })
    }

    fn from_dir(root: Dir) -> Self {
        let profile = DriveProfile::read(&root);
        let mut profiles = HashMap::new();
        if let Ok(key) = drive::volume_key(&root) {
            profiles.insert(key, profile);
        }
        Self {
            root,
            root_path: None,
            clash_hints: Mutex::new(ClashHints::default()),
            name_checks: Default::default(),
            profile,
            forced: false,
            profiles: Mutex::new(profiles),
            read_back: Mutex::default(),
            #[cfg(feature = "fault-plan")]
            faults: Faults::default(),
            checks: Mutex::default(),
            fill_hook: None,
        }
    }

    /// For tests: a file filled in under its real name is copied `chunk` bytes at a time, and
    /// `after_chunk` runs after each piece with the bytes copied so far (to act as another program
    /// in between, or fail as a drive would by returning an error).
    #[doc(hidden)]
    #[must_use]
    pub fn with_fill_hook(
        mut self,
        chunk: usize,
        after_chunk: impl Fn(u64) -> io::Result<()> + Send + Sync + 'static,
    ) -> Self {
        self.fill_hook = Some(fill::FillHook {
            chunk,
            after_chunk: std::sync::Arc::new(after_chunk),
        });
        self
    }

    /// Tests the drive this place is on, once, for a way to name a finished file that never
    /// replaces anything (see [`Naming`]): on PCTwin's own probe files, which are removed
    /// afterwards. Asked again, it answers from that test.
    pub fn check_drive(&self) -> Result<DriveCheck, GateError> {
        self.check_for(&self.root, "")
    }

    /// The drive test for the folder `dir` (at the stored path `folder`), run there the first
    /// time a file on that drive is named.
    fn check_for(&self, dir: &Dir, folder: &str) -> Result<DriveCheck, GateError> {
        if let Some(forced) = self.profile.forced_check() {
            return Ok(forced);
        }
        let key = drive::volume_key(dir)?;
        // Held through the test, so this place never tests a drive twice at once.
        let mut checks = self.checks.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(check) = checks.get(&key) {
            return Ok(*check);
        }
        let folder_path = || self.folder_path(dir, folder);
        let check = naming::for_system(
            naming::probe(dir, &folder_path, move_no_replace)?,
            self.profile_of(dir).kind(),
            Platform::host(),
        );
        checks.insert(key, check);
        Ok(check)
    }

    /// Forgets every drive test (after a handle went stale: the drive was removed, or another
    /// put in its place), so the next file named tests again.
    fn forget_checks(&self) {
        self.checks
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
    }

    /// The file name a file sent as `sent` is stored under in the folder `dir` (on that folder's
    /// drive), and every change made to its folders' and its own names on the way.
    fn converted(
        &self,
        sent: &IncomingPath,
        dir: &Dir,
    ) -> Result<(String, Vec<NameChange>), GateError> {
        let rules = self.profile_of(dir).name_rules();
        let mut changes: Vec<NameChange> = Vec::new();
        for part in sent.components() {
            for c in convert_name(part, rules).changes {
                if !changes.contains(&c) {
                    changes.push(c);
                }
            }
        }
        let name = sent
            .components()
            .last()
            .map(|n| convert_name(n, rules).name)
            .ok_or(GateError::Path(PathError::Empty))?;
        Ok((name, changes))
    }

    /// Refuses a file bigger than the drive of `dir` holds, before anything is made.
    fn check_fits(&self, dir: &Dir, announced: u64) -> Result<(), GateError> {
        match self.profile_of(dir).largest_file() {
            Some(largest) if announced > largest => Err(GateError::TooBigForDrive { largest }),
            _ => Ok(()),
        }
    }

    /// For tests (feature `fault-plan`): fake `plan`'s faults on this destination.
    #[cfg(feature = "fault-plan")]
    pub fn with_faults(mut self, plan: FaultPlan) -> Self {
        self.faults.plan = plan;
        self
    }

    /// Whether files written here can be read back from the drive past this laptop's cache,
    /// tested once (at the start of a move) with a small probe file written, read back and
    /// removed. Where they cannot, every file is still read back and checked, through the cache,
    /// and its result says so.
    pub fn read_back_check(&self) -> CheckedFrom {
        let volume = self
            .root
            .try_clone()
            .ok()
            .and_then(|d| identity(&d.into_std_file()).ok())
            .map(|(id, _)| id.volume);
        self.read_back_mode(volume, &self.root)
    }

    /// The tested answer for the drive numbered `volume` (tested in `dir` if not yet known).
    fn read_back_mode(&self, volume: Option<u64>, dir: &Dir) -> CheckedFrom {
        let known = |v| {
            self.read_back
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .get(&v)
                .copied()
        };
        if let Some(found) = volume.and_then(known) {
            return found;
        }
        let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let probe_name = temp_name(&format!("probe-{}-{n}", std::process::id()));
        let found = read_back::probe(dir, &probe_name, self.uncached_refused());
        if let Some(v) = volume {
            self.read_back
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .insert(v, found);
        }
        found
    }

    fn read_back_known(&self) -> HashMap<u64, CheckedFrom> {
        self.read_back
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// A reader on the file `name` in `dir` (the very file `held` is, when the drive can tell
    /// files apart) that reads past the cache where this drive allows it.
    fn read_back_of(
        &self,
        dir: &Dir,
        name: &str,
        held: &std::fs::File,
        pages_kept_out: bool,
    ) -> io::Result<ReadBack> {
        let id = identity(held).ok().map(|(id, _)| id);
        let mut from = self.read_back_mode(id.map(|i| i.volume), dir);
        if !pages_kept_out && from == CheckedFrom::Drive {
            // macOS: the file's pages may still be cached, and a read past the cache takes them.
            from = CheckedFrom::Memory(WhyMemory::CannotSkipCache);
        }
        let file = match from {
            CheckedFrom::Drive => {
                match read_back::open_uncached(dir, name, self.uncached_refused()) {
                    Ok(file) => file,
                    // Tested fine for this drive but refused now: still checked, and said so.
                    Err(_) => {
                        from = CheckedFrom::Memory(WhyMemory::CannotSkipCache);
                        dir.open(name)?.into_std()
                    }
                }
            }
            CheckedFrom::Memory(_) => dir.open(name)?.into_std(),
        };
        // The drive passed, but this very file may still be read through memory: then it is read
        // through an ordinary handle (some drives refuse a direct read of such a file).
        let file = match (from, self.this_file_from_drive(&file)) {
            (CheckedFrom::Drive, Err(why)) => {
                from = CheckedFrom::Memory(why);
                drop(file);
                dir.open(name)?.into_std()
            }
            _ => file,
        };
        // Never check some other file put under the same name meanwhile.
        if let (Some(id), Ok((found, _))) = (id, identity(&file))
            && found != id
        {
            return Err(io::Error::other("a different file is there now"));
        }
        ReadBack::new(file, from, self.flip_on_read())
    }

    #[cfg(feature = "fault-plan")]
    fn this_file_from_drive(&self, file: &std::fs::File) -> Result<(), WhyMemory> {
        if self.faults.plan.served_from_memory {
            return Err(WhyMemory::NotShown);
        }
        read_back::this_file_from_drive(file)
    }

    #[cfg(not(feature = "fault-plan"))]
    fn this_file_from_drive(&self, file: &std::fs::File) -> Result<(), WhyMemory> {
        read_back::this_file_from_drive(file)
    }

    /// As [`this_file_from_drive`](Self::this_file_from_drive), after the last read.
    #[cfg(feature = "fault-plan")]
    fn still_from_drive(&self, file: &std::fs::File) -> Result<(), WhyMemory> {
        if self.faults.plan.stored_specially_while_read {
            return Err(WhyMemory::StoredSpecially);
        }
        self.this_file_from_drive(file)
    }

    #[cfg(not(feature = "fault-plan"))]
    fn still_from_drive(&self, file: &std::fs::File) -> Result<(), WhyMemory> {
        self.this_file_from_drive(file)
    }

    #[cfg(feature = "fault-plan")]
    fn uncached_refused(&self) -> bool {
        self.faults.plan.uncached_refused
    }

    #[cfg(not(feature = "fault-plan"))]
    fn uncached_refused(&self) -> bool {
        false
    }

    #[cfg(feature = "fault-plan")]
    fn flip_on_read(&self) -> Option<u64> {
        match self.faults.plan.flip {
            Some(Flip::OnRead { at }) => Some(at),
            _ => None,
        }
    }

    #[cfg(not(feature = "fault-plan"))]
    fn flip_on_read(&self) -> Option<u64> {
        None
    }

    /// For tests: whether this write's first byte is to be stored flipped.
    #[cfg(feature = "fault-plan")]
    fn flip_this_write(&self) -> bool {
        match self.faults.plan.flip {
            Some(Flip::OnEveryWrite) => true,
            Some(Flip::OnWriteOnce) => !self.faults.flipped_once.swap(true, Ordering::Relaxed),
            _ => false,
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
        let (file_name, folders) = path
            .components()
            .split_last()
            .ok_or(GateError::Path(PathError::Empty))?;
        // The drive the file lands on (the deepest folder already there: any folder still to be
        // made is made on it) decides the largest file, before anything is made.
        self.check_fits(&self.walk_folders(folders)?.0, announced)?;
        let mut changes = Vec::new();
        let mut shown = Vec::new();
        let mut created = Vec::new();
        let mut dir = self.root.try_clone()?;
        for folder in folders {
            // Each name follows the rules of the drive it is made on.
            let converted = convert_name(folder, self.profile_of(&dir).name_rules());
            changes.extend(converted.changes.iter().copied());
            let (opened, made) = open_or_create_folder(&dir, &converted.name)?;
            dir = opened;
            shown.push(converted.name);
            if made {
                created.push(shown.join("/"));
            }
        }
        let converted = convert_name(file_name, self.profile_of(&dir).name_rules());
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
        let file = file.into_std();
        let pages_kept_out = read_back::keep_pages_out(&file);
        let file = File::from_std(file);
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
            mark: None,
            created,
            read_back: None,
            pages_kept_out,
        })
    }

    /// The folder (inside the approved folder, `/` between folders) a file sent as `path` lands
    /// in, with names converted exactly as for writing (each for the drive it is made on).
    /// Nothing is made and no link is followed.
    pub fn folder_of(&self, path: &IncomingPath) -> String {
        let parts = path.components();
        let folders = &parts[..parts.len().saturating_sub(1)];
        match self.walk_folders(folders) {
            Ok((_, names)) => names.join("/"),
            // The approved folder cannot be looked at: its own drive's rules, for every name.
            Err(_) => folders
                .iter()
                .map(|f| convert_name(f, self.profile.name_rules()).name)
                .collect::<Vec<_>>()
                .join("/"),
        }
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

    /// Windows: removes the file at the stored path `stored` only if it is, right now, the very
    /// file `expect`, a regular file with one name, and `verify` (the caller's check, given the
    /// file opened for reading and held) says it is unchanged. Nothing is acted on by name after
    /// a check (security design notes, decided 8 October 2026): the file is opened relative to its
    /// folder with sharing that lets others only read, so while it is held nobody can write,
    /// rename, replace or remove it; it is checked on that handle, and that handle's file is
    /// removed (`fs_at`'s delete by handle). A file another program holds is
    /// [`Removed::InUse`]; a file stored online only is never opened for its bytes (so never
    /// downloaded) and is [`Removed::CloudOnly`].
    ///
    /// Linux and macOS have no removal by handle: there undo moves the name aside first, then
    /// checks ([`check_copy`](Self::check_copy), [`remove_checked`](Self::remove_checked);
    /// decided 9 October 2026). Only Linux removes, under a write lease; macOS keeps every copy
    /// (strengthened 13 October 2026).
    ///
    /// It takes an undo right (an open undo permit, or the ticket undo's close lends), so nothing
    /// can be removed once undo is closed for good (the type makes that impossible).
    /// `about_to_remove` is called once, after the last check and just before the removal (to
    /// record it first); nothing is removed if it fails. A file whose drive gives it no number (0)
    /// cannot be told apart from others, so it is never removed; nor is any file on a drive
    /// without hard links (FAT, exFAT), whose numbers do not stay with a file
    /// ([`Removed::Unsupported`]).
    #[cfg(windows)]
    pub fn remove_if_unchanged(
        &self,
        right: &impl pctwin_journal::UndoRight,
        stored: &str,
        expect: FileId,
        verify: impl FnOnce(&mut std::fs::File) -> io::Result<bool>,
        about_to_remove: impl FnOnce() -> io::Result<()>,
    ) -> Result<Removed, GateError> {
        if !right.in_force() {
            return Err(GateError::Io(io::Error::new(
                io::ErrorKind::TimedOut,
                "undo stopped here: it took too long",
            )));
        }
        // No handle PCTwin itself still holds on a file it made may look like another program's.
        birth_hold::settle();
        let Some((dir, name)) = self.open_stored_folder(stored)? else {
            return Ok(Removed::Gone);
        };
        if is_undo_name(name) || is_temp_name(name) {
            return Err(GateError::Io(invalid("not a copy undo removes")));
        }
        let folder = stored.rsplit_once('/').map_or("", |(f, _)| f);
        // A drive without hard links (FAT, exFAT) numbers a file by where its entry sits in its
        // folder, so another file made there later can carry the very number recorded: the
        // number proves nothing, and the copy is kept (decided 15 October 2026). A drive that
        // cannot be tested now is not trusted either.
        if !self.check_for(&dir, folder).is_ok_and(|c| c.hard_links) {
            return Ok(Removed::Unsupported);
        }
        // The right is looked at again just before the removal (a lent right may have ended).
        let about_to_remove = || {
            if right.in_force() {
                about_to_remove()
            } else {
                Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "undo stopped here: it took too long",
                ))
            }
        };
        remove_checked(&dir, folder, name, expect, verify, about_to_remove)
    }

    /// Windows: whether a copy whose removal was under way when PCTwin stopped is still under its
    /// name ([`Left::Here`]) or gone (removal by handle leaves nothing part-way). Decided from the
    /// disk, before anything else.
    #[cfg(windows)]
    pub fn resume_removal(
        &self,
        _right: &impl pctwin_journal::UndoRight,
        stored: &str,
        expect: FileId,
    ) -> Result<Left, GateError> {
        let Some((_dir, name)) = self.open_stored_folder(stored)? else {
            return Ok(Left::Gone);
        };
        if is_undo_name(name) || is_temp_name(name) {
            return Err(GateError::Io(invalid("not a copy undo removes")));
        }
        Ok(match self.stat(stored)? {
            Some(now) if now.id == expect => Left::Here,
            _ => Left::Gone,
        })
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
        self.check_fits(&dir, announced)?;
        let mut options = OpenOptions::new();
        options.read(true).write(true);
        let file = dir.open_with(temp_name, &options)?.into_std();
        let pages_kept_out = read_back::keep_pages_out(&file);
        let file = File::from_std(file);
        let (name, changes) = self.converted(sent, &dir)?;
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
            mark: None,
            created: Vec::new(),
            read_back: None,
            pages_kept_out,
        })
    }

    /// After a restart: the sealed temporary file at the stored path `temp` (every byte checked
    /// and on disk before the crash), to give the real name a file sent as `sent` gets. Only a
    /// regular file with a PCTwin temporary name is accepted. Dropped without a name, it is kept
    /// (never removed), so no early return can lose a checked copy.
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
        let (name, changes) = self.converted(sent, &dir)?;
        let held = dir.open(temp_name)?.into_std();
        if !held.metadata()?.is_file() {
            return Err(not_temp());
        }
        Ok(Sealed {
            destination: self,
            dir,
            temp: Some(temp_name.to_string()),
            folder: temp.rsplit_once('/').map_or("", |(f, _)| f).to_string(),
            name,
            sent_path: sent.original().to_string(),
            changes,
            // A checked copy found again after a restart: never thrown away by being dropped,
            // whichever way naming it ends (the journal's clean-up decides).
            kept_if_dropped: true,
            held: Some(held),
            mark_not_kept: None,
            checked_from: None,
            mark: None,
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
        let (file_name, folders) = path.components().split_last()?;
        let mut dir = self.root.try_clone().ok()?;
        let mut shown = Vec::new();
        for folder in folders {
            let name = convert_name(folder, self.profile_of(&dir).name_rules()).name;
            if !dir.symlink_metadata(&name).ok()?.is_dir() {
                return None;
            }
            dir = dir.open_dir(&name).ok()?;
            shown.push(name);
        }
        let name = convert_name(file_name, self.profile_of(&dir).name_rules()).name;
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

    /// Opens a stored file (by its stored path) to read it back from the drive, past this
    /// laptop's cache where the drive allows it ([`ReadBack::from`] says which). Never follows a
    /// link. Recovery proves a file found after a restart through this same reader.
    pub fn open_read_back(&self, stored: &str) -> io::Result<ReadBack> {
        let held = self.open_read(stored)?;
        let not_found = || io::Error::new(io::ErrorKind::NotFound, "not a stored file");
        let (dir, name) = self.open_stored_folder(stored)?.ok_or_else(not_found)?;
        self.read_back_of(&dir, name, &held, true)
    }

    /// The first name for `name` in `dir` that nothing uses now: `name` itself, or a numbered one
    /// such as `name (2).ext`. Nothing is claimed: another program can still take it first, so a
    /// claim never replaces anything and a taken name is simply tried again.
    fn free_name(&self, dir: &Dir, folder: &str, name: &str) -> Result<(String, bool), GateError> {
        let name_taken = |dir: &Dir, name: &str| {
            self.name_checks
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            name_taken(dir, name)
        };
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
        let key = hints().key(
            folder,
            &format!("{base}{ext}"),
            self.profile_of(dir).tells_case_apart(),
        );
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
/// Returns `None` when the name is taken. How is decided by the drive's test (`check`), never by
/// an error number alone: a hard link where the drive has them, else a move the drive itself
/// refuses if the name exists. Nothing is ever put under `name` but the whole file, and no
/// placeholder is made. A drive without either way is not named here at all.
fn claim(
    dir: &Dir,
    name: &str,
    temp: &str,
    folder_path: FolderPath<'_>,
    check: &DriveCheck,
) -> Result<Option<Linked>, GateError> {
    if check.naming != Naming::Atomic {
        return Err(GateError::NoSafeName);
    }
    if check.hard_links {
        // The whole file appears under its name at once, or not at all. The temporary name
        // stays until the journal has recorded the real one.
        match dir.hard_link(temp, dir, name) {
            Ok(()) => return Ok(Some(Linked::Hard)),
            Err(e) if is_taken(dir, name, &e) => return Ok(None),
            Err(e) if is_name_error(&e) => return Err(GateError::NameRejected),
            Err(e) if !check.no_replace_move => return Err(GateError::Io(e)),
            Err(_) => {}
        }
    }
    match move_no_replace(dir, temp, name, folder_path) {
        naming::Moved::Done => Ok(Some(Linked::Moved)),
        naming::Moved::Taken => Ok(None),
        // The drive's no-replace move passed its test, so an "invalid" now is the name.
        naming::Moved::Invalid | naming::Moved::Name => Err(GateError::NameRejected),
        // The drive no longer does what its test found (another drive in its place?).
        naming::Moved::Unsupported => Err(GateError::NoSafeName),
        // Windows answers a folder in the way with "access denied": the name is taken.
        naming::Moved::Failed(_) if dir.symlink_metadata(name).is_ok() => Ok(None),
        naming::Moved::Failed(e) => Err(GateError::Io(e)),
    }
}

/// An error that is about the name itself (too long, or characters the drive cannot hold).
fn is_name_error(e: &io::Error) -> bool {
    #[cfg(unix)]
    {
        use rustix::io::Errno;
        e.raw_os_error().is_some_and(|code| {
            [Errno::NAMETOOLONG, Errno::ILSEQ]
                .iter()
                .any(|n| n.raw_os_error() == code)
        })
    }
    #[cfg(not(unix))]
    {
        // ERROR_INVALID_NAME, ERROR_FILENAME_EXCED_RANGE.
        matches!(e.raw_os_error(), Some(123 | 206))
    }
}

/// Whether an error says the drive's handle went stale (the drive was removed, or another put
/// in its place): what was learnt about the drive no longer holds.
fn is_stale(e: &GateError) -> bool {
    let GateError::Io(e) = e else {
        return false;
    };
    #[cfg(unix)]
    {
        use rustix::io::Errno;
        e.raw_os_error().is_some_and(|code| {
            [Errno::STALE, Errno::NODEV, Errno::NXIO, Errno::IO]
                .iter()
                .any(|n| n.raw_os_error() == code)
        })
    }
    #[cfg(not(unix))]
    {
        // ERROR_NOT_READY, ERROR_DEV_NOT_EXIST, ERROR_DEVICE_NOT_CONNECTED.
        matches!(e.raw_os_error(), Some(21 | 55 | 1167))
    }
}

/// Moves PCTwin's own file `temp` onto `name` in `dir` only if nothing has `name`; the drive
/// checks and moves in one step, so another file can never be replaced, on a drive that honours
/// the request (which its test found). Only ever used for PCTwin's own files.
#[cfg(unix)]
fn move_no_replace(
    dir: &Dir,
    temp: &str,
    name: &str,
    _folder_path: FolderPath<'_>,
) -> naming::Moved {
    use naming::Moved;
    use rustix::fs::{RenameFlags, renameat_with};
    use rustix::io::Errno;
    // renameat2(RENAME_NOREPLACE) on Linux, renameatx_np(RENAME_EXCL) on macOS.
    match renameat_with(dir, temp, dir, name, RenameFlags::NOREPLACE) {
        Ok(()) => Moved::Done,
        Err(Errno::EXIST) => Moved::Taken,
        Err(Errno::INVAL) => Moved::Invalid,
        // Two numbers on macOS, one on Linux.
        Err(e) if e == Errno::NOSYS || e == Errno::NOTSUP || e == Errno::OPNOTSUPP => {
            Moved::Unsupported
        }
        Err(Errno::NAMETOOLONG | Errno::ILSEQ) => Moved::Name,
        Err(e) => Moved::Failed(e.into()),
    }
}

/// Windows: `MoveFileExW` without `MOVEFILE_REPLACE_EXISTING` (through `tempfile`, which wraps it
/// safely; it also clears the temporary file's attributes, which are PCTwin's own), on the
/// folder's full path, proven to be the folder held. Without such a path (a place opened through
/// the admin helper) there is no such move.
#[cfg(windows)]
fn move_no_replace(
    _dir: &Dir,
    temp: &str,
    name: &str,
    folder_path: FolderPath<'_>,
) -> naming::Moved {
    use naming::Moved;
    let Ok(folder) = folder_path() else {
        return Moved::Unsupported;
    };
    let mut source = match tempfile::TempPath::try_from_path(folder.join(temp)) {
        Ok(source) => source,
        Err(e) => return Moved::Failed(e),
    };
    // Never removed by `tempfile`, whatever happens.
    source.disable_cleanup(true);
    match source.persist_noclobber(folder.join(name)) {
        Ok(()) => Moved::Done,
        Err(e) if e.error.kind() == io::ErrorKind::AlreadyExists => Moved::Taken,
        Err(e) => match e.error.raw_os_error() {
            // ERROR_FILE_EXISTS, ERROR_ALREADY_EXISTS.
            Some(80 | 183) => Moved::Taken,
            // ERROR_INVALID_NAME, ERROR_FILENAME_EXCED_RANGE.
            Some(123 | 206) => Moved::Name,
            // ERROR_INVALID_FUNCTION, ERROR_NOT_SUPPORTED.
            Some(1 | 50) => Moved::Unsupported,
            _ => Moved::Failed(e.error),
        },
    }
}

#[cfg(not(any(unix, windows)))]
fn move_no_replace(
    _dir: &Dir,
    _temp: &str,
    _name: &str,
    _folder_path: FolderPath<'_>,
) -> naming::Moved {
    naming::Moved::Unsupported
}

pub use pctwin_journal::{Born, DownloadMark, FileId};

/// A stored file's size, modified time and identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stat {
    pub len: u64,
    pub modified: Option<std::time::SystemTime>,
    pub id: FileId,
}

/// A file's number on its drive, refused when it is 0 (drives use 0 for "no number", so such a
/// file cannot be told apart from another).
fn number(index: u64) -> io::Result<std::num::NonZeroU64> {
    std::num::NonZeroU64::new(index).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::Unsupported,
            "this drive gives files no number, so PCTwin cannot tell them apart",
        )
    })
}

/// Which file this is, and how many names it has. On Linux and macOS the drive is told by its
/// file-system number (the same however the drive is plugged in, unlike the device number), the
/// file by its inode number, and the birth time is kept where the drive keeps it (Linux asks for
/// it explicitly and gets none where the drive has none, never a made-up one).
#[cfg(unix)]
fn identity(file: &std::fs::File) -> io::Result<(FileId, u64)> {
    use std::os::unix::fs::MetadataExt;
    let meta = file.metadata()?;
    let volume = rustix::fs::fstatvfs(file)?.f_fsid;
    Ok((
        FileId {
            volume,
            index: number(meta.ino())?,
            born: meta.created().ok().map(Born::of),
        },
        meta.nlink(),
    ))
}

/// Which file `file` is on its drive, the way the gate tells files apart everywhere (for other
/// crates that must compare with what the gate recorded, such as the old laptop's answer).
pub fn file_identity(file: &std::fs::File) -> io::Result<FileId> {
    Ok(identity(file)?.0)
}

#[cfg(windows)]
fn identity(file: &std::fs::File) -> io::Result<(FileId, u64)> {
    let info = winapi_util::file::information(file)?;
    Ok((
        FileId {
            volume: info.volume_serial_number(),
            index: number(info.file_index())?,
            born: None,
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
    let _ = name;
    #[cfg(windows)]
    {
        let mut options = OpenOptions::new();
        options.write(true);
        if let Ok(file) = dir.open_with(name, &options) {
            let _ = file.sync_all();
        }
    }
    if let Ok(folder) = folder_to_flush(dir) {
        let _ = folder.sync_all();
    }
}

/// A handle on the folder itself that a flush works through. The folder's own handle cannot be
/// used on Linux: it is opened for paths only, and flushing it fails ("bad file descriptor"), so
/// the folder is opened again for reading. On Windows a folder opens for writing with backup
/// semantics.
fn folder_to_flush(dir: &Dir) -> io::Result<std::fs::File> {
    #[cfg(unix)]
    {
        use rustix::fs::{Mode, OFlags, openat};
        let fd = openat(
            dir,
            ".",
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
            Mode::empty(),
        )?;
        Ok(std::fs::File::from(fd))
    }
    #[cfg(windows)]
    {
        /// Lets Windows open a folder as a handle.
        const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
        let mut folder = OpenOptions::new();
        folder.write(true);
        cap_std::fs::OpenOptionsExt::custom_flags(&mut folder, FILE_FLAG_BACKUP_SEMANTICS);
        Ok(dir.open_with(".", &folder)?.into_std())
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = dir;
        Err(io::ErrorKind::Unsupported.into())
    }
}

/// What a held file's count of names says before it is removed: none left means another try (or
/// the person) removed it meanwhile, so it is gone; more than one means another name was made for
/// it, which stays the person's. `None` is the one name, the only case that may go ahead.
fn by_names(links: u64) -> Option<Removed> {
    match links {
        0 => Some(Removed::Gone),
        1 => None,
        _ => Some(Removed::Linked),
    }
}

/// Flushes a folder's list of names (after a name left it), so a removal is on the disk before
/// the journal says it is done. It goes through `dir`, the very folder held (opened again
/// relative to it, never by path). Only a drive that says it cannot flush folders at all is let
/// go; on Linux and macOS a folder that cannot be opened for the flush, or a flush that fails, is
/// an error. (Windows keeps its own rule: a folder it cannot open for writing is let go.)
fn sync_folder(dir: &Dir) -> io::Result<()> {
    let failed = |e: io::Error| {
        io::Error::other(format!(
            "it was removed, but the drive did not confirm the change was written ({e})"
        ))
    };
    let folder = match folder_to_flush(dir) {
        Ok(folder) => folder,
        Err(_) if cfg!(windows) => return Ok(()),
        Err(e) => return Err(failed(e)),
    };
    match folder.sync_all() {
        Ok(()) => Ok(()),
        Err(e) if cannot_flush_folders(&e) => Ok(()),
        Err(e) => Err(failed(e)),
    }
}

/// A drive that does not flush folders at all, as opposed to a flush that failed: on Linux and
/// macOS only the system's own "invalid for this kind of file" (EINVAL) or "not supported"
/// (ENOTSUP, EOPNOTSUPP) from the flush; on Windows "not supported" or "not that kind of file".
fn cannot_flush_folders(e: &io::Error) -> bool {
    #[cfg(unix)]
    {
        use rustix::io::Errno;
        e.raw_os_error().is_some_and(|code| {
            [Errno::INVAL, Errno::NOTSUP, Errno::OPNOTSUPP]
                .iter()
                .any(|n| n.raw_os_error() == code)
        })
    }
    #[cfg(not(unix))]
    {
        matches!(
            e.kind(),
            io::ErrorKind::Unsupported | io::ErrorKind::InvalidInput
        ) || (cfg!(windows) && matches!(e.raw_os_error(), Some(1 | 50)))
    }
}

/// How removing a copy ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Removed {
    /// The very file, unchanged, is removed.
    Removed,
    /// Nothing is at the name.
    Gone,
    /// Something else is at the name (another file, a folder or a link): left as it is.
    NotThatFile,
    /// The very file, but changed since: left as it is.
    Changed,
    /// The file has more than one name on the drive: left as it is.
    Linked,
    /// Another program is using the file: left as it is, for another try.
    InUse,
    /// The file is stored online only: left as it is (never downloaded).
    CloudOnly,
    /// This drive cannot move a name without risking another file, so nothing was done.
    Unsupported,
    /// A database's working files, a lock file or an app's owner file is beside it, or it is a
    /// kind of file an app keeps open (a mail store, a password safe, a database): left as it is.
    AppKeepsOpen,
    /// The system cannot prove that no other program can still write to it (macOS, other
    /// systems without Linux write leases, a drive without leases): kept as it is, for the
    /// person to review.
    CannotCheck,
    /// Another file took its name meanwhile: it is kept, under the visible name `at` beside it.
    KeptBeside { at: String },
    /// Removed, but bytes written to it meanwhile (or a program opening it at that moment) made
    /// PCTwin save it again, under `at` beside its name.
    Salvaged { at: String },
}

/// Where a copy is whose removal was under way ([`Destination::resume_removal`]).
#[cfg(windows)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Left {
    /// Still under its name: it was not removed.
    Here,
    /// Not under its name: it was removed, or moved by the person.
    Gone,
}

/// Whether `name` is one of undo's own (a private name a copy's name is moved to, or a salvage
/// being written): never a name undo removes.
fn is_undo_name(name: &str) -> bool {
    name.starts_with(".pctwin-undo-") || name.starts_with(".pctwin-salvage-")
}

/// Tries for a file another program holds for a moment, and the wait before each.
#[cfg(windows)]
const IN_USE_WAITS_MS: [u64; 3] = [50, 100, 200];

#[cfg(windows)]
fn remove_checked(
    dir: &Dir,
    _folder: &str,
    name: &str,
    expect: FileId,
    verify: impl FnOnce(&mut std::fs::File) -> io::Result<bool>,
    about_to_remove: impl FnOnce() -> io::Result<()>,
) -> Result<Removed, GateError> {
    use cap_std::fs::MetadataExt as _;
    use cap_std::fs::OpenOptionsExt;
    use fs_at::os::windows::FileExt;
    use std::os::windows::fs::MetadataExt as _;
    const DELETE: u32 = 0x0001_0000;
    const FILE_READ_DATA: u32 = 0x0001;
    const FILE_READ_ATTRIBUTES: u32 = 0x0080;
    // For `fs_at` to clear a read-only mark where the drive needs that to remove the file.
    const FILE_WRITE_ATTRIBUTES: u32 = 0x0100;
    const SYNCHRONIZE: u32 = 0x0010_0000;
    const FILE_SHARE_READ: u32 = 0x0001;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    const SHARING_VIOLATION: i32 = 32;
    const ACCESS_DENIED: i32 = 5;
    let mut options = OpenOptions::new();
    options
        .access_mode(
            DELETE | FILE_READ_DATA | FILE_READ_ATTRIBUTES | FILE_WRITE_ATTRIBUTES | SYNCHRONIZE,
        )
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    let mut waits = IN_USE_WAITS_MS.iter();
    // One step, looked at and opened together, and taken again as a whole after a wait: the look
    // before the open meets the same moments as the open itself.
    let mut held = loop {
        // Before opening, from the folder's list: a file stored online only is never opened
        // (opening it for its bytes would download it).
        let refused = match dir.symlink_metadata(name) {
            Ok(meta) => {
                if let Some(kept) = not_plain(meta.file_attributes()) {
                    return Ok(kept);
                }
                if !meta.is_file() {
                    return Ok(Removed::NotThatFile);
                }
                match dir.open_with(name, &options) {
                    Ok(f) => break f.into_std(),
                    Err(e) => e,
                }
            }
            Err(e) => e,
        };
        if refused.kind() == io::ErrorKind::NotFound {
            return Ok(Removed::Gone);
        }
        match refused.raw_os_error() {
            // Held by another program (sharing), or refused while another try's removal of it is
            // under way: Windows answers "access denied" to every look at a file between the
            // moment its removal is asked for and the moment it leaves the folder. Waited out
            // and looked at again; a refusal that lasts is in use, or (access denied) an error.
            Some(SHARING_VIOLATION | ACCESS_DENIED) => match waits.next() {
                Some(ms) => std::thread::sleep(std::time::Duration::from_millis(*ms)),
                None if refused.raw_os_error() == Some(SHARING_VIOLATION) => {
                    return Ok(Removed::InUse);
                }
                None => return Err(GateError::Io(refused)),
            },
            _ => return Err(GateError::Io(refused)),
        }
    };
    // Everything from here is on the held handle.
    let meta = held.metadata()?;
    if let Some(kept) = not_plain(meta.file_attributes()) {
        return Ok(kept);
    }
    if !meta.is_file() {
        return Ok(Removed::NotThatFile);
    }
    let (id, links) = identity(&held)?;
    if id != expect {
        return Ok(Removed::NotThatFile);
    }
    if let Some(kept) = by_names(links) {
        return Ok(kept);
    }
    if !verify(&mut held)? {
        return Ok(Removed::Changed);
    }
    // Sharing that blocks writers does not block making another name for the file: read again on
    // the same handle, as late as possible.
    if let Some(kept) = by_names(identity(&held)?.1) {
        return Ok(kept);
    }
    about_to_remove()?;
    match held.delete_by_handle() {
        Ok(()) => {
            // The removal on disk before the journal says it is done.
            sync_folder(dir)?;
            Ok(Removed::Removed)
        }
        Err((_, e)) => Err(GateError::Io(e)),
    }
}

/// A file that is not a plain file on the drive: stored online only, or a link or other special
/// entry (a reparse point).
#[cfg(windows)]
fn not_plain(attributes: u32) -> Option<Removed> {
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
    const FILE_ATTRIBUTE_OFFLINE: u32 = 0x1000;
    const FILE_ATTRIBUTE_RECALL_ON_OPEN: u32 = 0x0004_0000;
    const FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS: u32 = 0x0040_0000;
    if attributes
        & (FILE_ATTRIBUTE_OFFLINE
            | FILE_ATTRIBUTE_RECALL_ON_OPEN
            | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)
        != 0
    {
        return Some(Removed::CloudOnly);
    }
    (attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0).then_some(Removed::NotThatFile)
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
        self.admit(id, place, destination)
    }

    /// For tests: as [`approve`](Self::approve), on a drive behaving as `profile` says.
    #[doc(hidden)]
    pub fn approve_with_profile(
        &mut self,
        id: &str,
        place: Approved,
        path: &Path,
        profile: DriveProfile,
    ) -> Result<(), GateError> {
        self.check_new(id)?;
        let destination = Destination::open(path)?.with_profile(profile);
        self.admit(id, place, destination)
    }

    /// For tests: approves an already opened place (one given a drive profile or a fill hook).
    #[doc(hidden)]
    pub fn approve_opened(
        &mut self,
        id: &str,
        place: Approved,
        destination: Destination,
    ) -> Result<(), GateError> {
        self.check_new(id)?;
        self.admit(id, place, destination)
    }

    /// Tests the place's drive once, when it is chosen (security design notes): a drive with no
    /// safe way to name files is refused here, with the reason, before anything is sent. A test
    /// that could not be run (a folder PCTwin may not write to, say) refuses nothing here, as
    /// before: the drive is tested again before its first file is named, and nothing is named
    /// untested.
    fn admit(
        &mut self,
        id: &str,
        place: Approved,
        destination: Destination,
    ) -> Result<(), GateError> {
        if destination
            .check_drive()
            .is_ok_and(|check| check.naming == Naming::NoSafeMethod)
        {
            return Err(GateError::NoSafeName);
        }
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
        self.admit(id, place, Destination::from_dir(root))
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
    let born = std::time::Instant::now();
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
    let dir = parent.open_dir(name)?;
    if created && let Ok(held) = dir.try_clone() {
        birth_hold::hold(&held.into_std_file(), born);
    }
    Ok((dir, created))
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
    // Readable too: a reopened file is read through it to check what it holds, and a drive
    // with no safe rename fills the real name from this very handle.
    options.read(true).write(true).create_new(true);
    for _ in 0..64 {
        let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let name = format!(".pctwin-{}-{n}.part", std::process::id());
        let born = std::time::Instant::now();
        match dir.open_with(&name, &options) {
            Ok(file) => {
                let file = file.into_std();
                birth_hold::hold(&file, born);
                return Ok((File::from_std(file), name));
            }
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
    /// The download marker the file should have had but could not keep, because the drive cannot
    /// hold one (FAT and exFAT drives, some network drives): the copy is there, and the report
    /// must say plainly that the system no longer knows it came from the internet. `None` when
    /// every marker was kept (or there was none).
    pub mark_not_kept: Option<DownloadMark>,
    /// How its contents were checked before it got its name: read back from the drive, or from
    /// this laptop's memory (and why), for the report's "more details". `None` for a file checked
    /// before a restart and proven again by recovery.
    pub checked_from: Option<CheckedFrom>,
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
    /// The download marker to keep, if the original had one.
    mark: Option<DownloadMark>,
    /// Folders made for this file (stored paths, outermost first).
    created: Vec<String>,
    /// Where the latest read-back that matched read from; `None` until one has, and again after
    /// anything changes the file. Only a matched file can be sealed and named.
    read_back: Option<CheckedFrom>,
    /// The file is written so its pages stay out of the cache where that matters (macOS).
    pages_kept_out: bool,
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
        self.read_back = None;
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
        self.read_back = None;
        #[cfg(feature = "fault-plan")]
        let flipped;
        #[cfg(feature = "fault-plan")]
        let buf = if self.destination.flip_this_write() {
            flipped = {
                let mut b = buf.to_vec();
                b[0] ^= 0xFF;
                b
            };
            &flipped[..]
        } else {
            buf
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
            self.read_back = None;
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

    /// Marks the file as from the internet when it is sealed, as its original was on the old
    /// laptop (see [`DownloadMark`]), before it gets its real name, so it never appears unmarked.
    pub fn keep_download_mark(&mut self, mark: DownloadMark) {
        self.mark = Some(mark);
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

    /// Reads the whole file back from the drive and checks it against the sender's block
    /// fingerprints (`hashes`, BLAKE3, one per block of `block_size` bytes, the last over its real
    /// length): flushes it, then re-reads it through a second, read-only handle that skips this
    /// laptop's cache where the drive allows it (tested once per drive, see
    /// [`Destination::read_back_check`]). Says which blocks did not match and where the bytes came
    /// from. Only after a check that matched can the file be sealed and named; anything that
    /// changes the file afterwards, or a later check that fails, undoes that.
    pub fn verify(
        &mut self,
        block_size: u64,
        hashes: &[[u8; 32]],
    ) -> Result<Verification, GateError> {
        self.read_back = None;
        if self.received != self.announced {
            return Err(GateError::SizeMismatch {
                announced: self.announced,
                received: self.received,
            });
        }
        if block_size == 0 || hashes.len() as u64 != self.announced.div_ceil(block_size) {
            return Err(GateError::BadBlockList);
        }
        let file = self.file.as_ref().ok_or(GateError::Conflict)?;
        file.sync_all()?;
        let held = file.try_clone()?.into_std();
        let temp = self.temp_name.as_deref().ok_or(GateError::Conflict)?;
        let mut reader =
            self.destination
                .read_back_of(&self.dir, temp, &held, self.pages_kept_out)?;
        let mut check = read_back::check_blocks(&mut reader, self.announced, block_size, hashes)?;
        // Judged again after the last read: a file whose storage changed while it was read (another
        // program compressed it) was not read from the drive throughout.
        check.from = reader.judged_after(|f| self.destination.still_from_drive(f));
        if check.passed() {
            self.read_back = Some(check.from);
        }
        Ok(check)
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
        // Only a file read back from the drive and matched gets this far; the read-back flushed
        // it, and nothing has changed it since.
        let checked_from = self.read_back.ok_or(GateError::NotReadBack)?;
        let file = self.file.take().ok_or(GateError::Conflict)?.into_std();
        let temp_name = self.temp_name.as_deref().ok_or(GateError::Conflict)?;
        let mut mark_not_kept = None;
        if let Some(mark) = self.mark {
            // Before the time: writing Windows' marker can change the file's modified time.
            let kept = self.destination.profile_of(&self.dir).tries_markers()
                && marks::put(&self.dir, temp_name, &file, mark)?;
            if !kept {
                mark_not_kept = Some(mark);
            }
        }
        if let Some(time) = self.modified {
            file.set_modified(time)?;
            file.sync_all()?;
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
            kept_if_dropped: false,
            held: Some(file),
            mark_not_kept,
            checked_from: Some(checked_from),
            mark: self.mark,
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
    /// Kept, not removed, if dropped without a name.
    kept_if_dropped: bool,
    /// The file itself, held open until it is kept under its real name: while it is held, the
    /// drive cannot give its number to another file.
    held: Option<std::fs::File>,
    /// The marker that could not be kept on this drive, if any.
    mark_not_kept: Option<DownloadMark>,
    /// Where its read-back read from (`None`: checked before a restart).
    checked_from: Option<CheckedFrom>,
    /// The download marker the file carries, for a copy filled in under its real name.
    mark: Option<DownloadMark>,
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
    pub fn claim_as(mut self, stored: &str) -> Claim<'d> {
        let name = match self.name_in_folder(stored) {
            Ok(name) => name,
            Err(e) => return Claim::Failed(e, self),
        };
        let Some(temp) = self.temp.clone() else {
            return Claim::Failed(GateError::Conflict, self);
        };
        let check = match self.destination.check_for(&self.dir, &self.folder) {
            Ok(check) => check,
            Err(e) => {
                if is_stale(&e) {
                    self.destination.forget_checks();
                }
                return Claim::Failed(e, self);
            }
        };
        let folder_path = || self.destination.folder_path(&self.dir, &self.folder);
        let linked = match claim(&self.dir, &name, &temp, &folder_path, &check) {
            Ok(Some(linked)) => linked,
            Ok(None) => return Claim::Taken(self),
            Err(e) => {
                if is_stale(&e) || matches!(e, GateError::NoSafeName) {
                    // What was learnt about the drive may no longer hold: test it again next time.
                    self.destination.forget_checks();
                }
                return Claim::Failed(e, self);
            }
        };
        self.temp = None;
        flush_name(&self.dir, &name);
        Claim::Named(self.claimed(name, (linked == Linked::Hard).then_some(temp)))
    }

    /// How this file's drive gives finished files their names (tested once per drive): record
    /// the step this calls for first, then [`claim_as`](Self::claim_as) (a hard link or a
    /// no-replace move) or [`fill_as`](Self::fill_as) (filled in under the name).
    pub fn naming(&self) -> Result<Naming, GateError> {
        Ok(self.destination.check_for(&self.dir, &self.folder)?.naming)
    }

    /// After a restart: the download marker this file carries (the journal's record of it), so a
    /// copy filled in under its real name gets it too.
    pub fn keep_download_mark(&mut self, mark: Option<DownloadMark>) {
        self.mark = mark;
    }

    /// For a drive with no safe rename: the tag a file filled in here gets at once (a fresh
    /// creation time, where the drive keeps one exactly), and how it is proven PCTwin's after a
    /// crash (its identity where the drive keeps identities, else that tag, else nothing past a
    /// restart). Record it, with the name, before [`fill_as`](Self::fill_as).
    pub fn fill_plan(&self) -> Result<FillPlan, GateError> {
        let check = self.destination.check_for(&self.dir, &self.folder)?;
        let born = check.exact_birth_time.then(fill::fresh_born);
        // Windows cannot read a drive's kind (it is unknown there unless a test gives one), but
        // only drives that keep identities have hard links.
        let kind = self.destination.profile.kind();
        let stable = kind.keeps_identities() || (cfg!(windows) && check.hard_links);
        let proof = if stable {
            Proof::StableId
        } else if born.is_some() {
            Proof::Token
        } else {
            Proof::SameMountId
        };
        Ok(FillPlan {
            tag: FillTag {
                born,
                hidden: false,
            },
            proof,
        })
    }

    /// For a drive with no safe rename (see the `fill` module notes): creates `stored` (a name in
    /// its own folder) only if nothing has it, tags it at once, fills it from this checked file,
    /// makes it durable, reads it back, checks the name still holds it, and only then shows it,
    /// recording each step through `steps`. Nothing that is not PCTwin's is ever touched or
    /// replaced. Something already at the name: [`Claim::Taken`], untouched. A failure before it
    /// is recorded filled removes PCTwin's own partial copy only while the name still holds it;
    /// this checked file always comes back. Where the drive has hard links, the temporary name
    /// stays until [`Claimed::keep`], as for any file.
    pub fn fill_as(
        mut self,
        stored: &str,
        plan: &FillPlan,
        steps: &mut dyn FillSteps,
    ) -> Claim<'d> {
        let name = match self.name_in_folder(stored) {
            Ok(name) => name,
            Err(e) => return Claim::Failed(e, self),
        };
        let Some(temp) = self.temp.clone() else {
            return Claim::Failed(GateError::Conflict, self);
        };
        match self.destination.check_for(&self.dir, &self.folder) {
            Ok(check) if check.naming == Naming::FillIn => {}
            Ok(_) => return Claim::Failed(GateError::NoSafeName, self),
            Err(e) => return Claim::Failed(e, self),
        }
        // Made only if nothing has the name; never through a link.
        let mut options = OpenOptions::new();
        options.read(true).write(true).create_new(true);
        let mut target = match self.dir.open_with(&name, &options) {
            Ok(f) => f.into_std(),
            Err(e) if is_taken(&self.dir, &name, &e) => return Claim::Taken(self),
            Err(e) if is_name_error(&e) => return Claim::Failed(GateError::NameRejected, self),
            Err(e) => return Claim::Failed(GateError::Io(e), self),
        };
        // Its pages kept out of this laptop's memory as it is written (macOS), so reading it back
        // past the cache reads the drive, as for every received file.
        let pages_kept_out = read_back::keep_pages_out(&target);
        let mut progress = fill::Progress {
            born: None,
            written: 0,
        };
        let filled = self.fill_into(
            &name,
            &mut target,
            plan,
            steps,
            &mut progress,
            pages_kept_out,
        );
        match filled {
            Ok(()) => {}
            Err((e, true)) => {
                // Recorded filled: complete and checked; what remains is finished after a
                // restart. Kept.
                return Claim::Failed(e, self);
            }
            Err((e, false)) => {
                // PCTwin's own partial copy goes, but only while the name still holds it.
                if fill::still_ours(&self.dir, &name, &target, &progress).unwrap_or(false) {
                    drop(target);
                    if self.dir.remove_file(&name).is_ok() {
                        let _ = sync_folder(&self.dir);
                        let _ = steps.cleared();
                    }
                }
                return Claim::Failed(e, self);
            }
        }
        drop(target);
        // The temporary file stays until the journal has committed ([`Claimed::keep`]).
        self.temp = None;
        Claim::Named(self.claimed(name, Some(temp)))
    }

    /// Steps 3 to 7 of a fill (see the `fill` module notes). On failure, whether it had been
    /// recorded filled.
    fn fill_into(
        &mut self,
        name: &str,
        target: &mut std::fs::File,
        plan: &FillPlan,
        steps: &mut dyn FillSteps,
        progress: &mut fill::Progress,
        pages_kept_out: bool,
    ) -> Result<(), (GateError, bool)> {
        let early = |e: GateError| (e, false);
        let io = |e: io::Error| (GateError::Io(e), false);
        if let Some(born) = plan.tag.born {
            fill::set_born(target, born).map_err(io)?;
            // Read back at once: a tag the drive did not keep exactly would prove nothing later.
            if fill::born_of(target) != Some(born) {
                return Err(early(GateError::Io(io::Error::other(
                    "this drive did not keep the mark PCTwin gives a file while it is copied",
                ))));
            }
            progress.born = Some(born);
        }
        let kept = fill::put_mark(
            &self.dir,
            name,
            target,
            self.mark,
            self.destination.profile.tries_markers(),
        )
        .map_err(early)?;
        if !kept && self.mark.is_some() {
            self.mark_not_kept = self.mark;
        }
        if let Ok((made, _)) = identity(target) {
            steps.made(made).map_err(io)?;
        }
        let held = self.held.as_mut().ok_or((GateError::Conflict, false))?;
        fill::copy_held(held, target, progress, self.destination.fill_hook.as_ref()).map_err(io)?;
        if progress.written != held.metadata().map_err(io)?.len() {
            return Err(early(GateError::Io(io::Error::other(
                "the checked copy changed while it was copied",
            ))));
        }
        fill::full_sync(target).map_err(io)?;
        // Read back through a second, read-only handle, past the cache where the drive allows it.
        let mut reader =
            match self
                .destination
                .read_back_of(&self.dir, name, target, pages_kept_out)
            {
                Ok(reader) => reader,
                // It refuses to read back another file put under the name: say so plainly.
                Err(_) if !fill::still_ours(&self.dir, name, target, progress).unwrap_or(false) => {
                    return Err(early(GateError::ReplacedWhileFilling));
                }
                Err(e) => return Err(io(e)),
            };
        if !steps.read_back(&mut reader).map_err(io)? {
            return Err(early(GateError::Io(io::Error::other(
                "the copy did not read back the same as the checked file",
            ))));
        }
        let from = reader.judged_after(|f| self.destination.still_from_drive(f));
        drop(reader);
        self.checked_from = Some(from);
        if !fill::still_ours(&self.dir, name, target, progress).map_err(io)? {
            return Err(early(GateError::ReplacedWhileFilling));
        }
        steps.filled().map_err(io)?;
        let late = |e: io::Error| (GateError::Io(e), true);
        if let Ok(time) = held.metadata().and_then(|m| m.modified()) {
            target.set_modified(time).map_err(late)?;
        }
        fill::full_sync(target).map_err(late)?;
        flush_name(&self.dir, name);
        Ok(())
    }

    /// Keeps the sealed file on disk under its temporary name for the journal to finish later
    /// (after a restart); dropping it instead removes it.
    pub fn persist(mut self) {
        self.temp = None;
    }

    /// Which file it is on its drive, to record before it gets its real name: after a crash only
    /// this very file is ever taken as it.
    pub fn identity(&self) -> io::Result<FileId> {
        let held = self
            .held
            .as_ref()
            .ok_or_else(|| io::Error::other("no file"))?;
        Ok(identity(held)?.0)
    }

    /// Gives the file a real name that nothing else uses, never replacing another file (without
    /// a journal: the name is not recorded first).
    pub fn claim(self) -> Result<Claimed<'d>, GateError> {
        let mut sealed = self;
        // Each try fails only if another program took the free name in between.
        for _ in 0..MAX_CLAIM_RACES {
            let name = sealed.next_name()?;
            let claim = if sealed.naming()? == Naming::FillIn {
                let plan = sealed.fill_plan()?;
                let source = sealed
                    .held
                    .as_ref()
                    .ok_or(GateError::Conflict)?
                    .try_clone()?;
                sealed.fill_as(&name, &plan, &mut fill::Unjournaled { source })
            } else {
                sealed.claim_as(&name)
            };
            match claim {
                Claim::Named(claimed) => return Ok(claimed),
                Claim::Taken(back) => sealed = back,
                Claim::Failed(e, _) => return Err(e),
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
            _held: self.held.take(),
            temp,
            finished: Finished {
                final_path: stored_path(&self.folder, &name),
                sent_path: std::mem::take(&mut self.sent_path),
                changes,
                mark_not_kept: self.mark_not_kept,
                checked_from: self.checked_from,
            },
            _destination: std::marker::PhantomData,
        }
    }
}

/// How giving a sealed file its real name ended. The sealed file always comes back unless it got
/// its name, so a failure never throws a checked file away.
pub enum Claim<'d> {
    /// It has its real name.
    Named(Claimed<'d>),
    /// Something took that name first: try another.
    Taken(Sealed<'d>),
    /// It could not be named now (the drive, say); it is still there, sealed.
    Failed(GateError, Sealed<'d>),
}

/// Most times a free name is taken by another program between finding it and claiming it before
/// giving up.
const MAX_CLAIM_RACES: u32 = 64;

impl Drop for Sealed<'_> {
    fn drop(&mut self) {
        // Never given its real name: never leave it behind (unless it is a checked copy found
        // again after a restart).
        drop(self.held.take());
        if self.kept_if_dropped {
            return;
        }
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
    /// The file, still held open until it is kept (so its number stays its own until then).
    _held: Option<std::fs::File>,
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
    pub fn keep(mut self) -> Finished {
        drop(self._held.take());
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

    /// A folder's own handle is opened for paths only on Linux, and flushing through it fails, so
    /// a flush that quietly did nothing there would go unseen. The handle a flush uses must
    /// really flush, on every system.
    /// Two tries at once on Unix: the slower one may hold the file after the faster removed its
    /// last name, and must then say it is gone, never that another name was made for it.
    #[test]
    fn a_file_with_no_names_left_is_gone() {
        assert_eq!(by_names(0), Some(Removed::Gone));
        assert_eq!(by_names(1), None);
        assert_eq!(by_names(2), Some(Removed::Linked));
    }

    #[test]
    fn a_folder_really_flushes() {
        let root = tempfile::tempdir().unwrap();
        let dir = Dir::open_ambient_dir(root.path(), cap_std::ambient_authority()).unwrap();
        folder_to_flush(&dir).unwrap().sync_all().unwrap();
        sync_folder(&dir).unwrap();
    }

    /// Unix: a folder that cannot be opened for its flush is an error, never a quiet "done"
    /// (security review, finding 5); only a drive that says it cannot flush folders at all is let go.
    #[cfg(unix)]
    #[test]
    fn a_folder_that_cannot_be_flushed_is_an_error() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let sub = root.path().join("locked");
        std::fs::create_dir(&sub).unwrap();
        let top = Dir::open_ambient_dir(root.path(), cap_std::ambient_authority()).unwrap();
        let dir = top.open_dir("locked").unwrap();
        std::fs::set_permissions(&sub, std::fs::Permissions::from_mode(0o300)).unwrap();
        let unreadable = std::fs::read_dir(&sub).is_err();
        let r = sync_folder(&dir);
        std::fs::set_permissions(&sub, std::fs::Permissions::from_mode(0o755)).unwrap();
        if unreadable {
            assert!(r.is_err(), "{r:?}");
        }
        use rustix::io::Errno;
        let e = |n: Errno| io::Error::from_raw_os_error(n.raw_os_error());
        assert!(cannot_flush_folders(&e(Errno::INVAL)));
        assert!(cannot_flush_folders(&e(Errno::NOTSUP)));
        assert!(cannot_flush_folders(&e(Errno::OPNOTSUPP)));
        for failed in [
            Errno::IO,
            Errno::NOSPC,
            Errno::ROFS,
            Errno::BADF,
            Errno::ACCESS,
        ] {
            assert!(!cannot_flush_folders(&e(failed)), "{failed:?}");
        }
    }

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
        assert!(matches!(
            move_no_replace(&dir, "t.part", "photo.jpg", &path_of(&root)),
            naming::Moved::Done
        ));
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
        assert!(matches!(
            move_no_replace(&dir, "gone.part", "doc.pdf", &path_of(&root)),
            naming::Moved::Failed(_)
        ));
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
                matches!(
                    move_no_replace(&dir, "t.part", taken, &path_of(&root)),
                    naming::Moved::Taken
                ),
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

    /// A drive whose test found a no-replace move that is not there now (here: Windows without
    /// the folder's full path) is an unsafe drive for this file, never a taken name: a taken name
    /// would send the file on to other names it can never get either.
    #[cfg(windows)]
    #[test]
    fn a_move_that_is_no_longer_there_is_never_taken_for_a_taken_name() {
        let (root, dir) = folder();
        std::fs::write(root.path().join("t.part"), b"abc").unwrap();
        let check = DriveCheck::forced(Naming::Atomic, false);
        let no_path = || -> io::Result<std::path::PathBuf> { Err(io::Error::other("none")) };
        assert!(matches!(
            claim(&dir, "a.txt", "t.part", &no_path, &check),
            Err(GateError::NoSafeName)
        ));
        assert!(root.path().join("t.part").exists());
        assert!(!root.path().join("a.txt").exists());
    }

    #[cfg(windows)]
    #[test]
    fn without_a_proven_folder_path_nothing_is_moved() {
        let (root, dir) = folder();
        std::fs::write(root.path().join("t.part"), b"abc").unwrap();
        let no_path = || -> io::Result<std::path::PathBuf> { Err(io::Error::other("none")) };
        assert!(matches!(
            move_no_replace(&dir, "t.part", "a.txt", &no_path),
            naming::Moved::Unsupported
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
}
