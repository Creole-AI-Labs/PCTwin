//! The product key finder (Product Spec: "surfaces legitimately stored license keys so you can
//! reactivate"; Task List 2.1, "licence and sign-in checklist; product key finder").
//!
//! **Only what an app keeps for this person, in the open.** The `licence-places` catalogue
//! (data) names where an app itself keeps its licence: a file in the person's own folders, a
//! plain-text value in the person's (or the laptop's) registry, a value in the person's own
//! preferences. Nothing else is read:
//!
//! - **no protected store**: never the Keychain, Credential Manager, DPAPI, Secret Service, a
//!   browser's saved logins, SAM or LSA (Security Design, Part C "Never", and Part G);
//! - **nothing decoded or decrypted**: a value is used only when it is plain text as stored (a
//!   registry value of the plain-text kind, a preference that is a string, a file that is plain
//!   text). Windows' own `DigitalProductId` is an encoded binary value, so it is never read; an
//!   app that obscures its key is not a place;
//! - **no memory reading, no running program asked**, nothing elevated: the reader runs as the
//!   person, read-only, on their own laptop, opens no link (a linked or reparse-point file is
//!   refused) and reads at most [`MAX_KEY_BYTES`].
//!
//! **Shown only to the person, on their own laptop** (Security Design A06, the secrets rules):
//! a key is a [`LicenceKey`], which cannot be serialised, prints as hidden in any debug output,
//! is wiped from memory when dropped, and is only ever handed out by [`LicenceKey::reveal`] for
//! the screen that shows it on request. [`find`] lists where keys are (the app and the place,
//! never the key); [`read_one`] reads one key again, live, when the person asks to see it. No
//! key goes in a report, a log, the move's record or anything the AI sees.

use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;

use pctwin_catalogue::{Catalogue, Row};
use pctwin_record::{AppSystem, apps::text_ok};
use serde::Deserialize;
use zeroize::Zeroizing;

/// Most bytes one key (or one licence file) may hold.
pub const MAX_KEY_BYTES: usize = 8 * 1024;
/// Most bytes of a preferences file read to find one value.
const MAX_PREFERENCES_BYTES: u64 = 4 * 1024 * 1024;

/// Which of the person's own folders a file is under.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Base {
    /// The person's home folder.
    Home,
    /// Their apps' settings folder: `AppData\Roaming` on Windows, `Library/Application Support`
    /// on a Mac, `.config` on Linux.
    AppData,
    /// Windows only: `AppData\Local`.
    LocalAppData,
}

/// Which registry tree a value is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Tree {
    /// The person's own (`HKEY_CURRENT_USER`).
    Person,
    /// The laptop's (`HKEY_LOCAL_MACHINE\SOFTWARE`), read as the person, never elevated.
    Machine,
}

/// Where an app keeps its licence.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(
    rename_all = "kebab-case",
    rename_all_fields = "kebab-case",
    tag = "kind",
    deny_unknown_fields
)]
pub enum Place {
    /// A plain-text file.
    File { under: Base, path: String },
    /// A plain-text registry value (Windows).
    Registry {
        tree: Tree,
        key: String,
        value: String,
    },
    /// A string in the person's own preferences (Mac).
    Preference { domain: String, key: String },
}

/// One place an app keeps its licence.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct LicencePlaceRow {
    pub id: String,
    /// The app's name, as its maker writes it.
    pub app: String,
    pub system: AppSystem,
    pub place: Place,
}

fn id_ok(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
}

/// A path the catalogue gives under one of the person's folders: relative, plain names only
/// (no `..`, no drive, no stream, no wildcard), `/`-separated.
pub fn relative_ok(p: &str) -> bool {
    !p.is_empty()
        && p.len() <= 260
        && !p.starts_with('/')
        && p.split('/').all(|c| {
            !c.is_empty()
                && c != "."
                && c != ".."
                && !c.ends_with('.')
                && !c.contains(['\\', ':', '*', '?', '"', '<', '>', '|'])
                && text_ok(c, 255)
        })
}

fn registry_key_ok(k: &str) -> bool {
    !k.is_empty()
        && k.len() <= 255
        && !k.starts_with('\\')
        && !k.ends_with('\\')
        && k.split('\\')
            .all(|c| !c.is_empty() && c != ".." && text_ok(c, 255))
}

fn domain_ok(d: &str) -> bool {
    d.len() <= 255
        && d.split('.').count() >= 2
        && d.split('.').all(|l| {
            !l.is_empty()
                && l.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        })
}

impl Row for LicencePlaceRow {
    const CATALOGUE: &'static str = "licence-places";
    const FIELDS: &'static [&'static str] = &["id", "app", "system", "place"];

    fn check(&self) -> Result<(), String> {
        if !id_ok(&self.id) {
            return Err("a row ID is short letters, digits and `.-_`".into());
        }
        if self.app.trim().is_empty() || !text_ok(&self.app, 128) {
            return Err("an app's name is plain text".into());
        }
        match (&self.place, self.system) {
            (Place::File { under, path }, system) => {
                if !relative_ok(path) {
                    return Err("a file is a plain path under one of the person's folders".into());
                }
                if *under == Base::LocalAppData && system != AppSystem::Windows {
                    return Err("only Windows has a local app data folder".into());
                }
            }
            (Place::Registry { key, value, .. }, AppSystem::Windows) => {
                if !registry_key_ok(key) || value.is_empty() || !text_ok(value, 255) {
                    return Err("a registry place is a plain key and value name".into());
                }
            }
            (Place::Preference { domain, key }, AppSystem::Mac) => {
                if !domain_ok(domain) || key.is_empty() || !text_ok(key, 255) {
                    return Err("a preference is a domain and a key".into());
                }
            }
            _ => return Err("a place of another system's kind".into()),
        }
        Ok(())
    }

    fn identity(&self) -> Option<String> {
        Some(self.id.clone())
    }
}

/// A licence key, held only to be shown to the person on request.
pub struct LicenceKey(Zeroizing<String>);

impl LicenceKey {
    /// The key, for the screen that shows it to the person who asked. Never for a report, a log,
    /// the move's record or the AI.
    pub fn reveal(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for LicenceKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("LicenceKey(hidden)")
    }
}

/// Text that can be a key as stored: UTF-8, within [`MAX_KEY_BYTES`], not blank, and no control
/// characters but line breaks and tabs (a licence file has several lines). Anything else (a
/// binary value, an encoded or obscured one) is not a key PCTwin reads.
pub fn as_key(bytes: &[u8]) -> Option<LicenceKey> {
    if bytes.len() > MAX_KEY_BYTES {
        return None;
    }
    let text = Zeroizing::new(std::str::from_utf8(bytes).ok()?.to_owned());
    // `text_ok` refuses every control character and hidden mark; line breaks and tabs are
    // allowed by turning them into spaces for the check only.
    let plain =
        !text.trim().is_empty() && text_ok(&text.replace(['\n', '\r', '\t'], " "), MAX_KEY_BYTES);
    plain.then(|| LicenceKey(Zeroizing::new(text.trim().to_owned())))
}

/// What a value read from a place turned out to be.
#[derive(Debug)]
pub enum Read {
    /// Nothing there.
    Absent,
    /// There, and plain text.
    Key(LicenceKey),
    /// There, but not plain text (binary, encoded, too long, a link): not read further.
    NotPlain,
}

/// Reads places on the person's own laptop.
pub trait Places {
    /// The bytes of the file at `path` under `base`, at most `max` (more is [`Read::NotPlain`]).
    fn file(&self, base: Base, path: &Path, max: usize) -> Read;
    fn registry(&self, tree: Tree, key: &str, value: &str) -> Read;
    fn preference(&self, domain: &str, key: &str) -> Read;
}

/// Where a key was found: the app and the place, never the key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyPlace {
    /// The catalogue row (to read it again when the person asks to see it).
    pub id: String,
    pub app: String,
    pub place: Place,
}

/// Every place on this laptop (a `system` one) where an app keeps a licence key in the open, by
/// `rows`. Keys are read only to see they are there and plain, then dropped (wiped).
pub fn find(
    rows: &Catalogue<LicencePlaceRow>,
    system: AppSystem,
    places: &dyn Places,
) -> Vec<KeyPlace> {
    rows.rows()
        .iter()
        .filter(|r| r.system == system)
        .filter(|r| matches!(read_row(r, places), Read::Key(_)))
        .map(|r| KeyPlace {
            id: r.id.clone(),
            app: r.app.clone(),
            place: r.place.clone(),
        })
        .collect()
}

/// The key at row `id`, read again now, for the person who asked to see it.
pub fn read_one(
    rows: &Catalogue<LicencePlaceRow>,
    system: AppSystem,
    id: &str,
    places: &dyn Places,
) -> Option<LicenceKey> {
    let row = rows
        .rows()
        .iter()
        .find(|r| r.id == id && r.system == system)?;
    match read_row(row, places) {
        Read::Key(k) => Some(k),
        _ => None,
    }
}

fn read_row(row: &LicencePlaceRow, places: &dyn Places) -> Read {
    match &row.place {
        Place::File { under, path } => {
            if !relative_ok(path) {
                return Read::NotPlain;
            }
            let rel: PathBuf = path.split('/').collect();
            places.file(*under, &rel, MAX_KEY_BYTES)
        }
        Place::Registry { tree, key, value } => places.registry(*tree, key, value),
        Place::Preference { domain, key } => places.preference(domain, key),
    }
}

/// The person's own laptop, read as them.
pub struct ThisLaptop {
    /// The person's home folder.
    pub home: PathBuf,
}

impl ThisLaptop {
    fn base(&self, base: Base) -> Option<PathBuf> {
        let h = &self.home;
        Some(match base {
            Base::Home => h.clone(),
            Base::AppData if cfg!(windows) => h.join("AppData").join("Roaming"),
            Base::AppData if cfg!(target_os = "macos") => {
                h.join("Library").join("Application Support")
            }
            Base::AppData => h.join(".config"),
            Base::LocalAppData if cfg!(windows) => h.join("AppData").join("Local"),
            Base::LocalAppData => return None,
        })
    }
}

/// Reads `rel` under `base` without following any link on the way: every step must be a plain
/// folder and the last a plain file.
pub fn read_plain_file(base: &Path, rel: &Path, max: usize) -> Read {
    if rel.components().any(|c| !matches!(c, Component::Normal(_))) {
        return Read::NotPlain;
    }
    let mut at = base.to_path_buf();
    let steps: Vec<_> = rel.components().collect();
    for (i, c) in steps.iter().enumerate() {
        at.push(c.as_os_str());
        let Ok(meta) = std::fs::symlink_metadata(&at) else {
            return Read::Absent;
        };
        let last = i + 1 == steps.len();
        if meta.file_type().is_symlink() || (last && !meta.is_file()) || (!last && !meta.is_dir()) {
            return Read::NotPlain;
        }
        if last && meta.len() > max as u64 {
            return Read::NotPlain;
        }
    }
    let Ok(f) = std::fs::File::open(&at) else {
        return Read::Absent;
    };
    let mut bytes = Zeroizing::new(Vec::new());
    use std::io::Read as _;
    if f.take(max as u64 + 1).read_to_end(&mut bytes).is_err() {
        return Read::Absent;
    }
    match as_key(&bytes) {
        Some(k) => Read::Key(k),
        None => Read::NotPlain,
    }
}

impl Places for ThisLaptop {
    fn file(&self, base: Base, path: &Path, max: usize) -> Read {
        match self.base(base) {
            Some(b) => read_plain_file(&b, path, max),
            None => Read::Absent,
        }
    }

    #[cfg(windows)]
    fn registry(&self, tree: Tree, key: &str, value: &str) -> Read {
        use winreg::RegKey;
        use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, REG_SZ};
        let (root, key) = match tree {
            Tree::Person => (RegKey::predef(HKEY_CURRENT_USER), key.to_owned()),
            Tree::Machine => (
                RegKey::predef(HKEY_LOCAL_MACHINE),
                format!("SOFTWARE\\{key}"),
            ),
        };
        let Ok(k) = root.open_subkey_with_flags(&key, KEY_READ) else {
            return Read::Absent;
        };
        let Ok(raw) = k.get_raw_value(value) else {
            return Read::Absent;
        };
        // Only the plain-text kind: never an expandable, multi-string or binary value.
        if raw.vtype != REG_SZ || raw.bytes.len() > MAX_KEY_BYTES * 2 + 2 {
            return Read::NotPlain;
        }
        let units: Vec<u16> = raw
            .bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|p| u16::from_le_bytes(*p))
            .take_while(|u| *u != 0)
            .collect();
        let text = Zeroizing::new(String::from_utf16(&units).unwrap_or_default());
        match as_key(text.as_bytes()) {
            Some(k) => Read::Key(k),
            None => Read::NotPlain,
        }
    }

    #[cfg(not(windows))]
    fn registry(&self, _: Tree, _: &str, _: &str) -> Read {
        Read::Absent
    }

    fn preference(&self, domain: &str, key: &str) -> Read {
        if !cfg!(target_os = "macos") || !domain_ok(domain) {
            return Read::Absent;
        }
        let rel = PathBuf::from("Library")
            .join("Preferences")
            .join(format!("{domain}.plist"));
        preference_in(&self.home, &rel, key)
    }
}

/// The string `key` in the preferences file `rel` under `home`, read without following a link.
pub fn preference_in(home: &Path, rel: &Path, key: &str) -> Read {
    let path = home.join(rel);
    match std::fs::symlink_metadata(&path) {
        Ok(m) if m.file_type().is_symlink() || !m.is_file() => return Read::NotPlain,
        Ok(m) if m.len() > MAX_PREFERENCES_BYTES => return Read::NotPlain,
        Ok(_) => {}
        Err(_) => return Read::Absent,
    }
    let Ok(value) = plist::Value::from_file(&path) else {
        return Read::NotPlain;
    };
    match value.as_dictionary().and_then(|d| d.get(key)) {
        None => Read::Absent,
        Some(plist::Value::String(s)) => match as_key(s.as_bytes()) {
            Some(k) => Read::Key(k),
            None => Read::NotPlain,
        },
        // Data, numbers, dictionaries: not a key as stored.
        Some(_) => Read::NotPlain,
    }
}

/// The rows compiled into this build.
pub fn shipped() -> &'static Catalogue<LicencePlaceRow> {
    static ROWS: OnceLock<Catalogue<LicencePlaceRow>> = OnceLock::new();
    ROWS.get_or_init(|| Catalogue::shipped(include_str!("../catalogues/licence-places.json")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn rows(json: &str) -> Catalogue<LicencePlaceRow> {
        Catalogue::shipped(&format!(
            r#"{{"format":1,"catalogue":"licence-places","version":1,"rows":[{json}]}}"#
        ))
    }

    struct Fake {
        files: HashMap<PathBuf, Vec<u8>>,
        registry: HashMap<String, Vec<u8>>,
    }

    impl Places for Fake {
        fn file(&self, _: Base, path: &Path, max: usize) -> Read {
            match self.files.get(path) {
                None => Read::Absent,
                Some(b) if b.len() > max => Read::NotPlain,
                Some(b) => as_key(b).map_or(Read::NotPlain, Read::Key),
            }
        }
        fn registry(&self, _: Tree, key: &str, value: &str) -> Read {
            match self.registry.get(&format!("{key}\\{value}")) {
                None => Read::Absent,
                Some(b) => as_key(b).map_or(Read::NotPlain, Read::Key),
            }
        }
        fn preference(&self, _: &str, _: &str) -> Read {
            Read::Absent
        }
    }

    const ROWS: &str = r#"{"id":"editor","app":"Editor","system":"windows","place":{"kind":"file","under":"app-data","path":"Editor/Local/License.txt"}},
        {"id":"archiver","app":"Archiver","system":"windows","place":{"kind":"registry","tree":"person","key":"Software\\Archiver","value":"Serial"}},
        {"id":"encoded","app":"Encoded","system":"windows","place":{"kind":"registry","tree":"machine","key":"Encoded","value":"Blob"}},
        {"id":"mac-editor","app":"Editor","system":"mac","place":{"kind":"file","under":"app-data","path":"Editor/Local/License.txt"}}"#;

    fn laptop() -> Fake {
        Fake {
            files: HashMap::from([(
                PathBuf::from("Editor").join("Local").join("License.txt"),
                b"----- BEGIN LICENSE -----\nPerson\nABCD-1234\n----- END LICENSE -----\n".to_vec(),
            )]),
            registry: HashMap::from([
                (
                    "Software\\Archiver\\Serial".into(),
                    b"ARCH-9999-0000".to_vec(),
                ),
                // An encoded value, as Windows' own DigitalProductId is: never a key.
                (
                    "Encoded\\Blob".into(),
                    vec![0xa4, 0, 0, 0, 3, 0, 0, 0, 0x30, 0x30],
                ),
            ]),
        }
    }

    #[test]
    fn keys_kept_in_the_open_are_found_by_place_never_by_value() {
        let c = rows(ROWS);
        assert!(c.report().is_clean(), "{:?}", c.report());
        let found = find(&c, AppSystem::Windows, &laptop());
        let ids: Vec<_> = found.iter().map(|k| k.id.as_str()).collect();
        // The encoded value and the other system's row are not found.
        assert_eq!(ids, ["editor", "archiver"]);
        // What is found says where, and holds no key.
        let shown = format!("{found:?}");
        assert!(!shown.contains("ABCD-1234") && !shown.contains("ARCH-9999"));
    }

    #[test]
    fn a_key_is_read_again_only_when_asked_and_never_printed() {
        let c = rows(ROWS);
        let k = read_one(&c, AppSystem::Windows, "archiver", &laptop()).unwrap();
        assert_eq!(k.reveal(), "ARCH-9999-0000");
        assert_eq!(format!("{k:?}"), "LicenceKey(hidden)");
        assert!(read_one(&c, AppSystem::Windows, "encoded", &laptop()).is_none());
        // Another system's row is not read here.
        assert!(read_one(&c, AppSystem::Windows, "mac-editor", &laptop()).is_none());
        assert!(read_one(&c, AppSystem::Windows, "nothing", &laptop()).is_none());
    }

    #[test]
    fn only_plain_text_is_a_key() {
        assert!(as_key(b"ABCD-EFGH").is_some());
        assert!(as_key(b"line one\r\nline two\t!").is_some());
        assert!(as_key(b"   \n").is_none());
        assert!(as_key(&[0xff, 0xfe, 0x41]).is_none());
        assert!(as_key(b"AB\x00CD").is_none());
        assert!(as_key("AB\u{202E}CD".as_bytes()).is_none());
        assert!(as_key(&vec![b'A'; MAX_KEY_BYTES + 1]).is_none());
        assert!(as_key(&vec![b'A'; MAX_KEY_BYTES]).is_some());
    }

    #[test]
    fn rows_that_could_reach_elsewhere_are_skipped() {
        for bad in [
            r#"{"id":"a","app":"A","system":"windows","place":{"kind":"file","under":"home","path":"../Other/key.txt"}}"#,
            r#"{"id":"a","app":"A","system":"windows","place":{"kind":"file","under":"home","path":"/etc/key"}}"#,
            r#"{"id":"a","app":"A","system":"windows","place":{"kind":"file","under":"home","path":"C:/key"}}"#,
            r#"{"id":"a","app":"A","system":"windows","place":{"kind":"file","under":"home","path":"key.txt:secret"}}"#,
            r#"{"id":"a","app":"A","system":"windows","place":{"kind":"file","under":"home","path":"A\\..\\key"}}"#,
            r#"{"id":"a","app":"A","system":"windows","place":{"kind":"file","under":"home","path":"*.key"}}"#,
            r#"{"id":"a","app":"A","system":"mac","place":{"kind":"file","under":"local-app-data","path":"key"}}"#,
            r#"{"id":"a","app":"A","system":"mac","place":{"kind":"registry","tree":"person","key":"X","value":"Y"}}"#,
            r#"{"id":"a","app":"A","system":"windows","place":{"kind":"registry","tree":"person","key":"\\X\\..\\Y","value":"Y"}}"#,
            r#"{"id":"a","app":"A","system":"windows","place":{"kind":"preference","domain":"com.a.b","key":"k"}}"#,
            r#"{"id":"a","app":"A","system":"windows","place":{"kind":"keychain","service":"x"}}"#,
        ] {
            let c = rows(bad);
            assert!(c.rows().is_empty(), "{bad}");
        }
    }

    #[test]
    fn a_real_file_is_read_without_following_links() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        std::fs::create_dir_all(base.join("Editor").join("Local")).unwrap();
        std::fs::write(
            base.join("Editor").join("Local").join("License.txt"),
            "KEY-1",
        )
        .unwrap();
        let rel = Path::new("Editor").join("Local").join("License.txt");
        assert!(matches!(read_plain_file(base, &rel, 100), Read::Key(k) if k.reveal() == "KEY-1"));
        assert!(matches!(
            read_plain_file(base, Path::new("Editor/None.txt"), 100),
            Read::Absent
        ));
        // Too big, a folder, a way out.
        std::fs::write(base.join("big.txt"), vec![b'A'; 200]).unwrap();
        assert!(matches!(
            read_plain_file(base, Path::new("big.txt"), 100),
            Read::NotPlain
        ));
        assert!(matches!(
            read_plain_file(base, Path::new("Editor"), 100),
            Read::NotPlain
        ));
        assert!(matches!(
            read_plain_file(base, Path::new("../x"), 100),
            Read::NotPlain
        ));
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(base.join("Editor"), base.join("Link")).unwrap();
            assert!(matches!(
                read_plain_file(
                    base,
                    &Path::new("Link").join("Local").join("License.txt"),
                    100
                ),
                Read::NotPlain
            ));
        }
    }

    #[test]
    fn a_preference_is_read_only_as_a_string() {
        let dir = tempfile::tempdir().unwrap();
        let mut d = plist::Dictionary::new();
        d.insert("Licence".into(), plist::Value::String("PREF-KEY".into()));
        d.insert("Blob".into(), plist::Value::Data(vec![1, 2, 3]));
        plist::Value::Dictionary(d)
            .to_file_binary(dir.path().join("com.example.app.plist"))
            .unwrap();
        let rel = Path::new("com.example.app.plist");
        assert!(
            matches!(preference_in(dir.path(), rel, "Licence"), Read::Key(k) if k.reveal() == "PREF-KEY")
        );
        assert!(matches!(
            preference_in(dir.path(), rel, "Blob"),
            Read::NotPlain
        ));
        assert!(matches!(
            preference_in(dir.path(), rel, "Missing"),
            Read::Absent
        ));
    }

    #[test]
    fn the_shipped_rows_load_clean() {
        assert!(shipped().report().is_clean(), "{:?}", shipped().report());
    }
}
