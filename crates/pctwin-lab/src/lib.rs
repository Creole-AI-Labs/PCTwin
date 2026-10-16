//! PCTwin test lab: synthetic "old laptops" (Task List 0.7; engineering notes 5.1).
//!
//! [`build`] fills a folder with a repeatable, made-up user profile: ordinary documents and
//! pictures of mixed sizes, plus every awkward case a real laptop can hold (see [`Case`]): Unicode
//! and case collisions, reserved and trailing-dot names, long paths, links (one pointing outside
//! the profile), read-only and locked files, and names that are not valid text. The same seed
//! always gives the same files with the same contents.
//!
//! Not every case can exist on every file system. A case the system refuses is listed in
//! [`Made::skipped`] with the reason, never silently dropped, so a test knows exactly what it
//! covered on each runner.
//!
//! [`snapshot`] records every name, size and content fingerprint under a folder without following
//! links, and [`compare`] lists any difference: the "old laptop is read-only" check runs a move and
//! compares the source before and after.
//!
//! Everything here is synthetic. It never reads or copies anyone's real files.

pub mod plants;

use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The kinds of thing a synthetic laptop holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Case {
    /// An ordinary document, picture, song or download.
    Ordinary,
    /// A file of 0 bytes.
    Empty,
    /// Sizes one byte either side of the smallest transfer block.
    BlockEdge,
    /// One file far bigger than a block.
    Big,
    /// The same name in composed and decomposed Unicode ("café" twice).
    UnicodeForms,
    /// Two names that differ only by case ("Report.txt", "report.txt").
    CaseCollision,
    /// A name Windows reserves (CON, NUL.txt, COM1, aux.log).
    Reserved,
    /// A name ending in a dot or a space, which Windows normally strips.
    TrailingDotOrSpace,
    /// Emoji, right-to-left, Chinese, Devanagari and combining marks.
    ScriptMix,
    /// A path longer than Windows' old 260-character limit.
    LongPath,
    /// A link to a file beside it, a link to nothing, and a link pointing outside the profile.
    Link,
    /// A file that cannot be written to.
    ReadOnly,
    /// A file another program holds open with no sharing (Windows).
    Locked,
    /// A name starting with a dot.
    Dotfile,
    /// A name that is not valid text (bytes on Linux, unpaired UTF-16 on Windows).
    NotText,
}

/// What to make.
#[derive(Debug, Clone, Copy)]
pub struct Spec {
    /// The same seed always makes the same laptop.
    pub seed: u64,
    /// How many ordinary files.
    pub ordinary_files: usize,
    /// The size of the one big file.
    pub big_file: u64,
}

impl Default for Spec {
    fn default() -> Self {
        Self {
            seed: 1,
            ordinary_files: 200,
            big_file: 8 * 1024 * 1024,
        }
    }
}

/// One thing made.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MadeItem {
    pub case: Case,
    /// The path under the root, as [`snapshot`] names it.
    pub path: String,
}

/// One case the file system refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Skipped {
    pub case: Case,
    pub what: String,
    pub why: String,
}

/// A synthetic laptop on disk.
#[derive(Debug)]
pub struct Made {
    pub root: PathBuf,
    /// Top-level folders put somewhere other than under `root` (see [`build_with`]).
    pub places: BTreeMap<String, PathBuf>,
    pub made: Vec<MadeItem>,
    pub skipped: Vec<Skipped>,
    /// Files held open with no sharing; dropping [`Made`] (or [`Made::release`]) lets them go.
    held: Vec<File>,
}

impl Made {
    /// The cases that exist on this file system.
    pub fn cases(&self) -> std::collections::BTreeSet<Case> {
        self.made.iter().map(|m| m.case).collect()
    }

    /// Lets go of locked files and makes read-only files writable again, so the folder can be
    /// removed.
    pub fn release(&mut self) -> io::Result<()> {
        self.held.clear();
        for m in self.made.iter().filter(|m| m.case == Case::ReadOnly) {
            let p = resolve(&self.root, &self.places, Path::new(&m.path));
            let mut perm = fs::metadata(&p)?.permissions();
            #[allow(clippy::permissions_set_readonly_false)]
            perm.set_readonly(false);
            fs::set_permissions(&p, perm)?;
        }
        Ok(())
    }
}

/// A small, fast, repeatable random source (xorshift64*). Not for anything secret.
pub(crate) struct Rng(u64);

impl Rng {
    pub(crate) fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    pub(crate) fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub(crate) fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }
}

/// Contents for a file: half the files repeat a sentence (they compress), half are random.
fn contents(rng: &mut Rng, size: u64) -> Vec<u8> {
    let size = usize::try_from(size).unwrap_or(usize::MAX);
    let mut out = Vec::with_capacity(size);
    if rng.below(2) == 0 {
        let line = format!("Synthetic PCTwin test text {}.\n", rng.next());
        while out.len() < size {
            out.extend_from_slice(line.as_bytes());
        }
        out.truncate(size);
    } else {
        while out.len() < size {
            out.extend_from_slice(&rng.next().to_le_bytes());
        }
        out.truncate(size);
    }
    out
}

/// On Windows, the long form of a path (`\\?\C:\...`), which allows long paths, trailing dots and
/// reserved names. Elsewhere, the path itself.
pub fn long(path: &Path) -> io::Result<PathBuf> {
    #[cfg(windows)]
    {
        let abs = std::path::absolute(path)?;
        let s = abs.as_os_str().to_string_lossy();
        if s.starts_with(r"\\?\") {
            return Ok(abs);
        }
        let mut long = OsString::from(r"\\?\");
        long.push(abs.as_os_str());
        Ok(PathBuf::from(long))
    }
    #[cfg(not(windows))]
    {
        Ok(path.to_path_buf())
    }
}

/// How [`snapshot`] names a path: its parts joined with `/`, each as text, or as `hex:` and its
/// raw bytes (Linux) or UTF-16 units (Windows) when it is not valid text.
pub fn name_of(rel: &Path) -> String {
    rel.components()
        .map(|c| part_name(c.as_os_str()))
        .collect::<Vec<_>>()
        .join("/")
}

fn part_name(part: &OsStr) -> String {
    if let Some(s) = part.to_str() {
        return s.to_string();
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let hex: String = part.as_bytes().iter().map(|b| format!("{b:02x}")).collect();
        format!("hex:{hex}")
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let hex: String = part.encode_wide().map(|u| format!("{u:04x}")).collect();
        format!("hex:{hex}")
    }
    #[cfg(not(any(unix, windows)))]
    {
        format!("lossy:{}", part.to_string_lossy())
    }
}

/// Where a path under the laptop really is: under its top-level folder's place if it has one,
/// else under `root`.
fn resolve(root: &Path, places: &BTreeMap<String, PathBuf>, rel: &Path) -> PathBuf {
    let mut parts = rel.components();
    if let Some(first) = parts.next()
        && let Some(place) = first.as_os_str().to_str().and_then(|f| places.get(f))
    {
        return place.join(parts.as_path());
    }
    root.join(rel)
}

struct Maker {
    root: PathBuf,
    places: BTreeMap<String, PathBuf>,
    made: Vec<MadeItem>,
    skipped: Vec<Skipped>,
    held: Vec<File>,
}

impl Maker {
    fn note_err(&mut self, case: Case, what: &str, e: &io::Error) {
        let why = if e.kind() == io::ErrorKind::AlreadyExists {
            "the file system treats it as the same name as one already made".to_string()
        } else {
            e.to_string()
        };
        self.skipped.push(Skipped {
            case,
            what: what.to_string(),
            why,
        });
    }

    /// Makes a file at `rel` (parts under the root). Returns whether it was made.
    fn file(&mut self, case: Case, rel: &[&OsStr], bytes: &[u8]) -> bool {
        let rel_path: PathBuf = rel.iter().collect();
        let what = name_of(&rel_path);
        let result = (|| -> io::Result<()> {
            let full = long(&resolve(&self.root, &self.places, &rel_path))?;
            if let Some(parent) = full.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut f = File::options().write(true).create_new(true).open(&full)?;
            f.write_all(bytes)?;
            f.sync_all()?;
            // Some systems accept a name but store another (Windows strips a trailing dot without
            // the long form; macOS may store another Unicode form): count it only if it reads
            // back under the name asked for.
            let listed = fs::read_dir(full.parent().unwrap_or(&full))?
                .filter_map(Result::ok)
                .any(|e| Some(e.file_name().as_os_str()) == full.file_name());
            if !listed {
                return Err(io::Error::other(
                    "the file system stored it under a different name",
                ));
            }
            Ok(())
        })();
        match result {
            Ok(()) => {
                self.made.push(MadeItem { case, path: what });
                true
            }
            Err(e) => {
                self.note_err(case, &what, &e);
                false
            }
        }
    }
}

fn os(s: &str) -> &OsStr {
    OsStr::new(s)
}

/// Builds a synthetic laptop profile under `root` (which must exist and should be empty).
pub fn build(root: &Path, spec: &Spec) -> io::Result<Made> {
    build_with(root, spec, &BTreeMap::new())
}

/// As [`build`], with some top-level folders put elsewhere: `places` maps a folder name
/// (`Documents`) to where it really is, as a laptop whose Documents live in OneDrive or on
/// another drive has it. Paths in the result are still named from the top-level folder
/// (`Documents/...`).
pub fn build_with(
    root: &Path,
    spec: &Spec,
    places: &BTreeMap<String, PathBuf>,
) -> io::Result<Made> {
    let mut rng = Rng::new(spec.seed);
    let mut m = Maker {
        root: root.to_path_buf(),
        places: places.clone(),
        made: Vec::new(),
        skipped: Vec::new(),
        held: Vec::new(),
    };

    // Ordinary files: mostly small, some medium, a few large, in the usual folders.
    const FOLDERS: [&str; 6] = [
        "Documents",
        "Pictures",
        "Music",
        "Videos",
        "Desktop",
        "Downloads",
    ];
    const EXT: [&str; 8] = ["docx", "pdf", "jpg", "png", "mp3", "txt", "xlsx", "zip"];
    for i in 0..spec.ordinary_files {
        let folder = FOLDERS[rng.below(FOLDERS.len() as u64) as usize];
        let sub = format!("Folder {}", rng.below(8));
        let name = format!("file {i:04}.{}", EXT[rng.below(EXT.len() as u64) as usize]);
        let size = match rng.below(10) {
            0..=5 => rng.below(4 * 1024),
            6..=8 => 4 * 1024 + rng.below(252 * 1024),
            _ => 256 * 1024 + rng.below(1792 * 1024),
        };
        let bytes = contents(&mut rng, size);
        m.file(Case::Ordinary, &[os(folder), os(&sub), os(&name)], &bytes);
    }

    m.file(Case::Empty, &[os("Documents"), os("empty.txt")], &[]);
    let block = 128 * 1024;
    for (name, size) in [
        ("edge-under.bin", block - 1),
        ("edge-exact.bin", block),
        ("edge-over.bin", block + 1),
    ] {
        let bytes = contents(&mut rng, size);
        m.file(Case::BlockEdge, &[os("Documents"), os(name)], &bytes);
    }
    let big = contents(&mut rng, spec.big_file);
    m.file(Case::Big, &[os("Videos"), os("holiday.mp4")], &big);

    // Unicode: composed then decomposed "café".
    m.file(
        Case::UnicodeForms,
        &[os("Documents"), os("caf\u{e9}.txt")],
        b"composed",
    );
    m.file(
        Case::UnicodeForms,
        &[os("Documents"), os("cafe\u{301}.txt")],
        b"decomposed",
    );

    m.file(
        Case::CaseCollision,
        &[os("Documents"), os("Report.txt")],
        b"upper",
    );
    m.file(
        Case::CaseCollision,
        &[os("Documents"), os("report.txt")],
        b"lower",
    );

    for name in ["CON", "NUL.txt", "COM1", "aux.log", "LPT9.doc"] {
        m.file(
            Case::Reserved,
            &[os("Documents"), os("Reserved"), os(name)],
            name.as_bytes(),
        );
    }
    m.file(
        Case::TrailingDotOrSpace,
        &[os("Documents"), os("notes.")],
        b"dot",
    );
    m.file(
        Case::TrailingDotOrSpace,
        &[os("Documents"), os("draft ")],
        b"space",
    );

    for name in [
        "\u{1F600} smile.txt",
        "\u{645}\u{644}\u{641} \u{639}\u{631}\u{628}\u{64A}.txt",
        "\u{4E2D}\u{6587}\u{6587}\u{4EF6}.txt",
        "\u{939}\u{93F}\u{928}\u{94D}\u{926}\u{940}.txt",
        "\u{5D0}\u{5D1}\u{5D2}\u{200F}.txt",
    ] {
        m.file(
            Case::ScriptMix,
            &[os("Documents"), os("Scripts"), os(name)],
            name.as_bytes(),
        );
    }

    let deep: Vec<String> = (0..8)
        .map(|i| format!("{i} a fairly long folder name to pass the old limit"))
        .collect();
    let mut parts: Vec<&OsStr> = vec![os("Documents")];
    parts.extend(deep.iter().map(|s| os(s)));
    parts.push(os("deep file.txt"));
    m.file(Case::LongPath, &parts, b"deep");

    m.file(Case::Dotfile, &[os(".config-synthetic")], b"dotfile");

    // Read-only.
    if m.file(
        Case::ReadOnly,
        &[os("Documents"), os("read-only.txt")],
        b"keep",
    ) {
        let p = long(&resolve(root, places, Path::new("Documents/read-only.txt")))?;
        let mut perm = fs::metadata(&p)?.permissions();
        perm.set_readonly(true);
        fs::set_permissions(&p, perm)?;
    }

    // Links.
    links(&mut m);

    // Locked (Windows: held open with no sharing; other systems have no such lock).
    locked(&mut m);

    // Not valid text.
    not_text(&mut m);

    Ok(Made {
        root: root.to_path_buf(),
        places: m.places,
        made: m.made,
        skipped: m.skipped,
        held: m.held,
    })
}

fn links(m: &mut Maker) {
    let dir = resolve(&m.root, &m.places, Path::new("Documents/Links"));
    let target_ok = m.file(
        Case::Link,
        &[os("Documents"), os("Links"), os("target.txt")],
        b"target",
    );
    // The target itself is an ordinary file; only the links count as link cases.
    if target_ok {
        m.made.pop();
    }
    let outside = if cfg!(windows) {
        r"C:\Windows\System32"
    } else {
        "/etc"
    };
    for (name, to) in [
        ("to-target", "target.txt"),
        ("to-nothing", "missing.txt"),
        ("to-outside", outside),
    ] {
        let what = format!("Documents/Links/{name}");
        let made = (|| -> io::Result<()> {
            let link = long(&dir.join(name))?;
            #[cfg(unix)]
            {
                std::os::unix::fs::symlink(to, &link)
            }
            #[cfg(windows)]
            {
                std::os::windows::fs::symlink_file(to, &link)
            }
            #[cfg(not(any(unix, windows)))]
            {
                let _ = (to, link);
                Err(io::Error::other("links are not supported here"))
            }
        })();
        match made {
            Ok(()) => m.made.push(MadeItem {
                case: Case::Link,
                path: what,
            }),
            Err(e) => m.note_err(Case::Link, &what, &e),
        }
    }
}

fn locked(m: &mut Maker) {
    let what = "Documents/locked.pst";
    if !m.file(
        Case::Locked,
        &[os("Documents"), os("locked.pst")],
        b"mail store",
    ) {
        return;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        let held = long(&resolve(
            &m.root,
            &m.places,
            Path::new("Documents/locked.pst"),
        ))
        .and_then(|p| File::options().read(true).share_mode(0).open(p));
        match held {
            Ok(f) => m.held.push(f),
            Err(e) => {
                m.made.pop();
                m.note_err(Case::Locked, what, &e);
            }
        }
    }
    #[cfg(not(windows))]
    {
        m.made.pop();
        m.skipped.push(Skipped {
            case: Case::Locked,
            what: what.to_string(),
            why: "this system has no locks that stop another program reading".into(),
        });
    }
}

fn not_text(m: &mut Maker) {
    #[cfg(unix)]
    let name: OsString = {
        use std::os::unix::ffi::OsStringExt;
        OsString::from_vec(b"caf\xe9 latin1.txt".to_vec())
    };
    #[cfg(windows)]
    let name: OsString = {
        use std::os::windows::ffi::OsStringExt;
        let mut units: Vec<u16> = "lone ".encode_utf16().collect();
        units.push(0xD800);
        units.extend(".txt".encode_utf16());
        OsString::from_wide(&units)
    };
    #[cfg(not(any(unix, windows)))]
    let name = OsString::from("not-text.txt");
    m.file(
        Case::NotText,
        &[os("Documents"), name.as_os_str()],
        b"not text",
    );
}

/// One thing under a snapshot's root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum Entry {
    Folder,
    File {
        size: u64,
        /// BLAKE3 of the contents, or `None` if the file could not be read (it is locked).
        blake3: Option<String>,
        read_only: bool,
    },
    Link {
        to: String,
    },
}

/// Every name under a folder with what it holds.
pub type Snapshot = BTreeMap<String, Entry>;

/// Records everything under `root`, never following links.
pub fn snapshot(root: &Path) -> io::Result<Snapshot> {
    let mut out = Snapshot::new();
    let base = long(root)?;
    walk(&base, Path::new(""), &mut out)?;
    Ok(out)
}

fn walk(base: &Path, rel: &Path, out: &mut Snapshot) -> io::Result<()> {
    for entry in fs::read_dir(base.join(rel))? {
        let entry = entry?;
        let rel = rel.join(entry.file_name());
        let meta = fs::symlink_metadata(base.join(&rel))?;
        let key = name_of(&rel);
        if meta.file_type().is_symlink() {
            let to = fs::read_link(base.join(&rel))?;
            out.insert(key, Entry::Link { to: name_of(&to) });
        } else if meta.is_dir() {
            out.insert(key, Entry::Folder);
            walk(base, &rel, out)?;
        } else {
            let blake3 = File::open(base.join(&rel)).ok().and_then(|mut f| {
                let mut h = blake3::Hasher::new();
                let mut buf = vec![0u8; 64 * 1024];
                loop {
                    match f.read(&mut buf) {
                        Ok(0) => break Some(h.finalize().to_hex().to_string()),
                        Ok(n) => {
                            h.update(&buf[..n]);
                        }
                        Err(_) => break None,
                    }
                }
            });
            out.insert(
                key,
                Entry::File {
                    size: meta.len(),
                    blake3,
                    read_only: meta.permissions().readonly(),
                },
            );
        }
    }
    Ok(())
}

/// Records several folders as one snapshot, each under its own name (`Documents/...`), the way a
/// laptop's special folders are named whatever drive or cloud folder they really live in.
pub fn snapshot_named(folders: &[(String, PathBuf)]) -> io::Result<Snapshot> {
    let mut out = Snapshot::new();
    for (name, path) in folders {
        if !path.exists() {
            continue;
        }
        out.insert(name.clone(), Entry::Folder);
        for (k, v) in snapshot(path)? {
            out.insert(format!("{name}/{k}"), v);
        }
    }
    Ok(out)
}

/// Every difference between two snapshots, one line each; empty when they match.
pub fn compare(before: &Snapshot, after: &Snapshot) -> Vec<String> {
    let mut out = Vec::new();
    for (k, v) in before {
        match after.get(k) {
            None => out.push(format!("gone: {k}")),
            Some(a) if a != v => out.push(format!("changed: {k}")),
            Some(_) => {}
        }
    }
    for k in after.keys().filter(|k| !before.contains_key(*k)) {
        out.push(format!("added: {k}"));
    }
    out
}
