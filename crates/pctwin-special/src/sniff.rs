//! What a file is, from its bytes (ADOPTION change 8: recognised by content, never by its name or
//! the app it belongs to).
//!
//! [`shape`] reads a file's first [`HEAD`] bytes and last [`TAIL`] bytes and says which format
//! they carry: database headers and their logs, key-value stores, password databases, key
//! material by its armour and inner header, system-protected blobs, mail stores, model weights,
//! programs, caches, and plain structured text. A file whose name says one thing and whose bytes
//! say another is what its bytes say. Nothing here opens, runs or decrypts anything.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

/// How much of a file's start is read to tell its format.
pub const HEAD: usize = 4096;
/// How much of its end is read (some formats put their mark there).
pub const TAIL: usize = 8;

/// A file's format, as its bytes show it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Shape {
    Empty,
    /// An SQLite database.
    Sqlite,
    /// An SQLite write-ahead log.
    SqliteWal,
    /// An SQLite write-ahead log's shared-memory index (rebuilt by SQLite from the log).
    SqliteShm,
    /// An SQLite rollback journal.
    SqliteJournal,
    /// An LMDB data file.
    LmdbData,
    /// An LMDB lock file (rebuilt by LMDB).
    LmdbLock,
    /// A LevelDB table file.
    LevelDbTable,
    /// A LevelDB `CURRENT` pointer naming its manifest.
    LevelDbCurrent {
        manifest: String,
    },
    /// A KeePass-format password database.
    PasswordDatabase,
    /// A KeePass-format XML key file.
    PasswordKeyFile,
    /// A private key. `encrypted`: whether a passphrase protects it (`None`: cannot tell).
    /// `hardware`: a stub for a key held on a security key.
    PrivateKey {
        encrypted: Option<bool>,
        hardware: bool,
    },
    /// A public key, a list of them, or a certificate.
    PublicKey,
    /// An OpenPGP secret key block.
    PgpPrivate,
    /// An OpenPGP public key block.
    PgpPublic,
    /// A GnuPG keybox (public keys).
    GpgKeybox,
    /// A GnuPG private key file. `shadowed`: a stub for a smartcard key; `protected`: under a
    /// passphrase.
    GpgPrivateKey {
        shadowed: bool,
        protected: bool,
    },
    /// A Windows DPAPI blob: protected by this laptop's account.
    DpapiBlob,
    /// An Outlook data file; `server_cache`: an offline copy of a server mailbox.
    OutlookData {
        server_cache: bool,
    },
    /// An mbox mail file.
    Mbox,
    /// GGUF model weights.
    Gguf,
    /// Safetensors model weights.
    Safetensors,
    /// A program, library or script.
    Executable,
    /// A cache directory tag (`CACHEDIR.TAG`, by its signature line).
    CacheTag,
    /// A browser-engine cache entry or index (by its magic number).
    CacheEntry,
    Json,
    Xml,
    PropertyList,
    Ini,
    /// Other text.
    Text,
    /// Other bytes.
    Binary,
}

impl Shape {
    /// A database or store an app writes while it runs (captured consistently, never raw).
    pub fn is_live_store(&self) -> bool {
        matches!(
            self,
            Shape::Sqlite | Shape::LmdbData | Shape::LevelDbTable | Shape::LevelDbCurrent { .. }
        )
    }
}

const SQLITE: &[u8] = b"SQLite format 3\0";
const SQLITE_JOURNAL: [u8; 8] = [0xd9, 0xd5, 0x05, 0xf9, 0x20, 0xa1, 0x63, 0xd7];
const DPAPI: [u8; 20] = [
    0x01, 0x00, 0x00, 0x00, 0xd0, 0x8c, 0x9d, 0xdf, 0x01, 0x15, 0xd1, 0x11, 0x8c, 0x7a, 0x00, 0xc0,
    0x4f, 0xc2, 0x97, 0xeb,
];
const KDBX: [u8; 4] = [0x03, 0xd9, 0xa2, 0x9a];
const LMDB_MAGIC: u32 = 0xBEEF_C0DE;
const LEVELDB_TABLE: u64 = 0xdb47_7524_8b80_fb57;
const SIMPLE_CACHE: u64 = 0xfcfb_6d1b_a772_5c30;
const CACHE_TAG: &[u8] = b"Signature: 8a477f597d28d172789f06886806bc55";

fn u32_le(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn u32_be(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn u64_le(b: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(b.get(at..at + 8)?.try_into().ok()?))
}

/// The format of a file of `len` bytes whose first bytes are `head` (up to [`HEAD`]) and last
/// bytes are `tail` (up to [`TAIL`]).
pub fn shape(head: &[u8], tail: &[u8], len: u64) -> Shape {
    if len == 0 || head.is_empty() {
        return Shape::Empty;
    }
    if let Some(s) = binary_shape(head, tail) {
        return s;
    }
    text_shape(head).unwrap_or(Shape::Binary)
}

fn binary_shape(head: &[u8], tail: &[u8]) -> Option<Shape> {
    if head.starts_with(SQLITE) {
        return Some(Shape::Sqlite);
    }
    if matches!(u32_be(head, 0), Some(0x377f_0682 | 0x377f_0683)) {
        return Some(Shape::SqliteWal);
    }
    if head.starts_with(&SQLITE_JOURNAL) {
        return Some(Shape::SqliteJournal);
    }
    // The shared-memory index: version 3007000 in the machine's own byte order, written twice.
    if head.len() >= 96 && u32_le(head, 0) == Some(3_007_000) && head[..48] == head[48..96] {
        return Some(Shape::SqliteShm);
    }
    if head.starts_with(&DPAPI) {
        return Some(Shape::DpapiBlob);
    }
    if head.starts_with(b"bplist0") {
        return Some(Shape::PropertyList);
    }
    if head.starts_with(&KDBX) && matches!(head.get(4..8), Some([0x65 | 0x67, 0xfb, 0x4b, 0xb5])) {
        return Some(Shape::PasswordDatabase);
    }
    if head.starts_with(b"!BDN") {
        return match head.get(8..10) {
            Some(b"SM") => Some(Shape::OutlookData {
                server_cache: false,
            }),
            Some(b"SO") => Some(Shape::OutlookData { server_cache: true }),
            _ => None,
        };
    }
    if head.starts_with(b"GGUF") && matches!(u32_le(head, 4), Some(1..=16)) {
        return Some(Shape::Gguf);
    }
    if let Some(n) = u64_le(head, 0)
        && (2..=100_000_000).contains(&n)
        && head.get(8) == Some(&b'{')
        && head.len() >= 16
        && head.get(9..).is_some_and(|r| {
            let r = &r[..r.len().min(64)];
            r.iter().all(|b| b.is_ascii())
        })
    {
        // A safetensors header is JSON right after its length; check its first key is quoted.
        let after = &head[9..];
        let first = after.iter().find(|b| !b.is_ascii_whitespace());
        if first == Some(&b'"') {
            return Some(Shape::Safetensors);
        }
    }
    if u32_le(head, 16) == Some(LMDB_MAGIC) {
        return Some(Shape::LmdbData);
    }
    if u32_le(head, 0) == Some(LMDB_MAGIC) {
        return Some(Shape::LmdbLock);
    }
    if tail.len() == TAIL && u64_le(tail, 0) == Some(LEVELDB_TABLE) {
        return Some(Shape::LevelDbTable);
    }
    if u64_le(head, 0) == Some(SIMPLE_CACHE)
        || matches!(u32_le(head, 0), Some(0xC103_CAC3 | 0xC104_CAC3))
    {
        return Some(Shape::CacheEntry);
    }
    if head.len() >= 12 && &head[8..12] == b"KBXf" {
        return Some(Shape::GpgKeybox);
    }
    if is_executable(head) {
        return Some(Shape::Executable);
    }
    None
}

fn is_executable(head: &[u8]) -> bool {
    // A Windows program: "MZ" and a "PE" header where it says.
    if head.starts_with(b"MZ")
        && let Some(at) = u32_le(head, 0x3c)
        && head.get(at as usize..at as usize + 4) == Some(b"PE\0\0")
    {
        return true;
    }
    if head.starts_with(b"\x7fELF") {
        return true;
    }
    // Mach-O, both byte orders and widths; a universal binary names a few architectures (a Java
    // class file shares its first bytes but carries a version in the hundreds there).
    let m = u32_be(head, 0);
    if matches!(
        m,
        Some(0xfeed_face | 0xfeed_facf | 0xcefa_edfe | 0xcffa_edfe)
    ) {
        return true;
    }
    if m == Some(0xcafe_babe) && u32_be(head, 4).is_some_and(|n| (1..=20).contains(&n)) {
        return true;
    }
    // A script that names its interpreter.
    head.starts_with(b"#!/") || head.starts_with(b"#! /")
}

fn text_shape(head: &[u8]) -> Option<Shape> {
    // A cut multi-byte character at the end of the head is still text.
    let text = match std::str::from_utf8(head) {
        Ok(t) => t,
        Err(e) if e.error_len().is_none() => std::str::from_utf8(&head[..e.valid_up_to()]).ok()?,
        Err(_) => return None,
    };
    if text.contains('\0') {
        return None;
    }
    let t = text.trim_start_matches('\u{feff}').trim_start();
    if t.starts_with(CACHE_TAG_STR) {
        return Some(Shape::CacheTag);
    }
    if let Some(s) = armoured(t) {
        return Some(s);
    }
    if let Some(s) = gpg_key(t) {
        return Some(s);
    }
    if public_key_lines(t) {
        return Some(Shape::PublicKey);
    }
    if let Some(name) = t.strip_prefix("MANIFEST-")
        && let Some(num) = name.strip_suffix('\n')
        && !num.is_empty()
        && num.bytes().all(|b| b.is_ascii_digit())
    {
        return Some(Shape::LevelDbCurrent {
            manifest: format!("MANIFEST-{num}"),
        });
    }
    if t.starts_with("From ") && mbox_like(t) {
        return Some(Shape::Mbox);
    }
    if t.starts_with('{') || t.starts_with('[') && json_like(t) {
        return Some(Shape::Json);
    }
    if t.starts_with('<') {
        return Some(xml_kind(t));
    }
    if t.starts_with("#!") {
        return Some(Shape::Executable);
    }
    if ini_like(t) {
        return Some(Shape::Ini);
    }
    Some(Shape::Text)
}

const CACHE_TAG_STR: &str = match std::str::from_utf8(CACHE_TAG) {
    Ok(s) => s,
    Err(_) => "",
};

/// `[` starts both JSON and an INI section: JSON when what follows is a value.
fn json_like(t: &str) -> bool {
    let rest = t[1..].trim_start();
    rest.starts_with(['{', '[', '"', ']'])
        || rest.starts_with(|c: char| c == '-' || c.is_ascii_digit())
        || rest.starts_with("true")
        || rest.starts_with("false")
        || rest.starts_with("null")
}

fn xml_kind(t: &str) -> Shape {
    let low: String = t.chars().take(512).collect::<String>().to_ascii_lowercase();
    if low.contains("<!doctype plist") || low.contains("<plist") {
        return Shape::PropertyList;
    }
    // A KeePass XML key file: its root element.
    let root = low
        .split('<')
        .map(str::trim_start)
        .find(|e| !e.is_empty() && !e.starts_with('?') && !e.starts_with('!'));
    if root.is_some_and(|r| r.starts_with("keyfile>") || r.starts_with("keyfile ")) {
        return Shape::PasswordKeyFile;
    }
    Shape::Xml
}

fn ini_like(t: &str) -> bool {
    let mut lines = t
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with(';') && !l.starts_with('#'));
    match lines.next() {
        Some(l) if l.starts_with('[') && l.ends_with(']') && l.len() > 2 => true,
        Some(l) => {
            l.split_once('=')
                .is_some_and(|(k, _)| !k.trim().is_empty() && !k.contains(' '))
                && lines.all(|l| l.contains('=') || (l.starts_with('[') && l.ends_with(']')))
        }
        None => false,
    }
}

fn mbox_like(t: &str) -> bool {
    // "From " then, on the next lines, mail headers ("Name: value").
    t.lines().skip(1).take(8).any(|l| {
        l.split_once(':').is_some_and(|(k, _)| {
            !k.is_empty() && k.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
    })
}

/// Key material in PEM or OpenSSH armour.
fn armoured(t: &str) -> Option<Shape> {
    let first = t.lines().next()?.trim();
    let label = first.strip_prefix("-----BEGIN ")?.strip_suffix("-----")?;
    Some(match label {
        "OPENSSH PRIVATE KEY" => openssh_private(t),
        "ENCRYPTED PRIVATE KEY" => Shape::PrivateKey {
            encrypted: Some(true),
            hardware: false,
        },
        "PRIVATE KEY" => Shape::PrivateKey {
            encrypted: Some(false),
            hardware: false,
        },
        l if l.ends_with(" PRIVATE KEY") => Shape::PrivateKey {
            // The older form says so in a header line.
            encrypted: Some(t.lines().take(4).any(|l| l.contains("ENCRYPTED"))),
            hardware: false,
        },
        "PGP PRIVATE KEY BLOCK" => Shape::PgpPrivate,
        "PGP PUBLIC KEY BLOCK" => Shape::PgpPublic,
        "CERTIFICATE"
        | "PUBLIC KEY"
        | "RSA PUBLIC KEY"
        | "TRUSTED CERTIFICATE"
        | "SSH2 PUBLIC KEY" => Shape::PublicKey,
        _ => return None,
    })
}

/// An OpenSSH private key: its inner header names the cipher (none: no passphrase) and the key
/// type (`sk-`: held on a security key; the file is only a handle to it).
fn openssh_private(t: &str) -> Shape {
    let body: String = t
        .lines()
        .skip(1)
        .take_while(|l| !l.starts_with("-----END"))
        .flat_map(|l| l.trim().chars())
        .collect();
    let unknown = Shape::PrivateKey {
        encrypted: None,
        hardware: false,
    };
    let Ok(bin) = STANDARD.decode(body.as_bytes()) else {
        return unknown;
    };
    let Some(mut r) = bin.strip_prefix(b"openssh-key-v1\0") else {
        return unknown;
    };
    let mut string = || -> Option<&[u8]> {
        let n = u32_be(r, 0)? as usize;
        let s = r.get(4..4 + n)?;
        r = &r[4 + n..];
        Some(s)
    };
    let Some(cipher) = string() else {
        return unknown;
    };
    let encrypted = Some(cipher != b"none");
    let (Some(_kdf), Some(_opts)) = (string(), string()) else {
        return unknown;
    };
    // The count of keys, then the first public key, whose first field is its type.
    let hardware = r
        .get(4..)
        .and_then(|rest| {
            let n = u32_be(rest, 0)? as usize;
            let key = rest.get(4..4 + n)?;
            let tn = u32_be(key, 0)? as usize;
            key.get(4..4 + tn)
        })
        .is_some_and(|ty| ty.starts_with(b"sk-"));
    Shape::PrivateKey {
        encrypted,
        hardware,
    }
}

/// A GnuPG private key file (an S-expression, or its newer text form with a `Key:` line).
fn gpg_key(t: &str) -> Option<Shape> {
    let head: String = t.chars().take(256).collect();
    let sexp = head.starts_with('(');
    let ext = head.lines().any(|l| l.starts_with("Key: ("));
    if !sexp && !ext {
        return None;
    }
    let shadowed = head.contains("shadowed-private-key");
    let protected = head.contains("protected-private-key");
    if shadowed || protected || head.contains("private-key") {
        Some(Shape::GpgPrivateKey {
            shadowed,
            protected,
        })
    } else {
        None
    }
}

/// Lines of OpenSSH public keys (`authorized_keys`, `known_hosts`, a `.pub` file).
fn public_key_lines(t: &str) -> bool {
    let mut any = false;
    for l in t
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        let mut words = l.split_whitespace();
        // known_hosts lines start with the host, then the key type.
        let key_at = |w: &str| {
            w.starts_with("ssh-")
                || w.starts_with("ecdsa-sha2-")
                || w.starts_with("sk-ssh-")
                || w.starts_with("sk-ecdsa-")
        };
        let ok = match (words.next(), words.next()) {
            (Some(a), Some(b)) if key_at(a) => b.starts_with("AAAA"),
            (Some(_), Some(b)) if key_at(b) => words.next().is_some_and(|c| c.starts_with("AAAA")),
            _ => false,
        };
        if !ok {
            return false;
        }
        any = true;
    }
    any
}

#[cfg(test)]
mod tests {
    use super::*;

    fn of(bytes: &[u8]) -> Shape {
        let head = &bytes[..bytes.len().min(HEAD)];
        let tail = &bytes[bytes.len().saturating_sub(TAIL)..];
        shape(head, tail, bytes.len() as u64)
    }

    fn ssh_body(cipher: &str, key_type: &str) -> String {
        let mut b = b"openssh-key-v1\0".to_vec();
        let mut put = |s: &[u8]| {
            b.extend_from_slice(&(s.len() as u32).to_be_bytes());
            b.extend_from_slice(s);
        };
        put(cipher.as_bytes());
        put(b"none");
        put(b"");
        let mut key = Vec::new();
        key.extend_from_slice(&(key_type.len() as u32).to_be_bytes());
        key.extend_from_slice(key_type.as_bytes());
        key.extend_from_slice(&[0, 0, 0, 1, 9]);
        let mut b2 = b;
        b2.extend_from_slice(&1u32.to_be_bytes());
        b2.extend_from_slice(&(key.len() as u32).to_be_bytes());
        b2.extend_from_slice(&key);
        format!(
            "-----BEGIN OPENSSH PRIVATE KEY-----\n{}\n-----END OPENSSH PRIVATE KEY-----\n",
            STANDARD.encode(b2)
        )
    }

    #[test]
    fn databases_and_their_logs_are_told_by_their_headers() {
        let mut db = SQLITE.to_vec();
        db.resize(200, 0);
        assert_eq!(of(&db), Shape::Sqlite);
        assert_eq!(of(&[0x37, 0x7f, 0x06, 0x82, 0, 0]), Shape::SqliteWal);
        assert_eq!(of(&[0x37, 0x7f, 0x06, 0x83, 1]), Shape::SqliteWal);
        let mut j = SQLITE_JOURNAL.to_vec();
        j.push(0);
        assert_eq!(of(&j), Shape::SqliteJournal);
        let mut shm = vec![0u8; 128];
        shm[..4].copy_from_slice(&3_007_000u32.to_le_bytes());
        shm[48..52].copy_from_slice(&3_007_000u32.to_le_bytes());
        assert_eq!(of(&shm), Shape::SqliteShm);
        // The same start, but the two copies differ: not the index.
        shm[60] = 1;
        assert_ne!(of(&shm), Shape::SqliteShm);
        let mut lmdb = vec![0u8; 64];
        lmdb[16..20].copy_from_slice(&LMDB_MAGIC.to_le_bytes());
        assert_eq!(of(&lmdb), Shape::LmdbData);
        let mut lock = vec![0u8; 64];
        lock[..4].copy_from_slice(&LMDB_MAGIC.to_le_bytes());
        assert_eq!(of(&lock), Shape::LmdbLock);
        let mut table = vec![1u8; 100];
        table.extend_from_slice(&LEVELDB_TABLE.to_le_bytes());
        assert_eq!(of(&table), Shape::LevelDbTable);
        assert_eq!(
            of(b"MANIFEST-000012\n"),
            Shape::LevelDbCurrent {
                manifest: "MANIFEST-000012".into()
            }
        );
        assert!(Shape::Sqlite.is_live_store() && !Shape::SqliteWal.is_live_store());
    }

    #[test]
    fn a_name_never_decides_only_the_bytes_do() {
        // The caller has a name; shape never sees it. A text file stays text.
        assert_eq!(of(b"hello there"), Shape::Text);
        assert_eq!(of(b""), Shape::Empty);
        assert_eq!(of(&[0xff, 0xfe, 0x00, 0x01]), Shape::Binary);
    }

    #[test]
    fn key_material_is_told_by_armour_and_inner_header() {
        assert_eq!(
            of(ssh_body("none", "ssh-ed25519").as_bytes()),
            Shape::PrivateKey {
                encrypted: Some(false),
                hardware: false
            }
        );
        assert_eq!(
            of(ssh_body("aes256-ctr", "ssh-ed25519").as_bytes()),
            Shape::PrivateKey {
                encrypted: Some(true),
                hardware: false
            }
        );
        // T25-029: a security-key handle, not a private key.
        assert_eq!(
            of(ssh_body("none", "sk-ssh-ed25519@openssh.com").as_bytes()),
            Shape::PrivateKey {
                encrypted: Some(false),
                hardware: true
            }
        );
        assert_eq!(
            of(b"-----BEGIN OPENSSH PRIVATE KEY-----\n!!!\n-----END OPENSSH PRIVATE KEY-----\n"),
            Shape::PrivateKey {
                encrypted: None,
                hardware: false
            }
        );
        assert_eq!(
            of(b"-----BEGIN RSA PRIVATE KEY-----\nProc-Type: 4,ENCRYPTED\nDEK-Info: x\n\nAAAA\n"),
            Shape::PrivateKey {
                encrypted: Some(true),
                hardware: false
            }
        );
        assert_eq!(
            of(b"-----BEGIN EC PRIVATE KEY-----\nAAAA\n"),
            Shape::PrivateKey {
                encrypted: Some(false),
                hardware: false
            }
        );
        assert_eq!(
            of(b"-----BEGIN ENCRYPTED PRIVATE KEY-----\nAAAA\n"),
            Shape::PrivateKey {
                encrypted: Some(true),
                hardware: false
            }
        );
        assert_eq!(
            of(b"-----BEGIN PRIVATE KEY-----\nAAAA\n"),
            Shape::PrivateKey {
                encrypted: Some(false),
                hardware: false
            }
        );
        assert_eq!(of(b"-----BEGIN CERTIFICATE-----\nAAAA\n"), Shape::PublicKey);
        assert_eq!(
            of(b"-----BEGIN PGP PRIVATE KEY BLOCK-----\n\n"),
            Shape::PgpPrivate
        );
        assert_eq!(
            of(b"-----BEGIN PGP PUBLIC KEY BLOCK-----\n\n"),
            Shape::PgpPublic
        );
        assert_eq!(of(b"-----BEGIN SOMETHING ELSE-----\n"), Shape::Text);
        assert_eq!(
            of(b"ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIFake person@laptop\n"),
            Shape::PublicKey
        );
        assert_eq!(
            of(b"example.org ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIFake\n"),
            Shape::PublicKey
        );
        // T25-031: a smartcard stub and a protected key, by their S-expression.
        assert_eq!(
            of(b"(20:shadowed-private-key(3:rsa(1:n3:abc)))"),
            Shape::GpgPrivateKey {
                shadowed: true,
                protected: false
            }
        );
        assert_eq!(
            of(b"Created: 20260101T000000\nKey: (protected-private-key (rsa (n #00#)))\n"),
            Shape::GpgPrivateKey {
                shadowed: false,
                protected: true
            }
        );
        let mut kbx = vec![0u8; 8];
        kbx.extend_from_slice(b"KBXf");
        kbx.extend_from_slice(&[0; 20]);
        assert_eq!(of(&kbx), Shape::GpgKeybox);
    }

    #[test]
    fn protected_stores_and_mail_are_told_by_their_magic() {
        let mut d = DPAPI.to_vec();
        d.extend_from_slice(&[7; 40]);
        assert_eq!(of(&d), Shape::DpapiBlob);
        let mut k = KDBX.to_vec();
        k.extend_from_slice(&[0x67, 0xfb, 0x4b, 0xb5, 0, 0, 4, 0]);
        assert_eq!(of(&k), Shape::PasswordDatabase);
        let mut k1 = KDBX.to_vec();
        k1.extend_from_slice(&[0x65, 0xfb, 0x4b, 0xb5]);
        assert_eq!(of(&k1), Shape::PasswordDatabase);
        assert_eq!(
            of(b"<?xml version=\"1.0\"?>\n<KeyFile><Meta><Version>2.0</Version></Meta></KeyFile>"),
            Shape::PasswordKeyFile
        );
        assert_eq!(
            of(b"!BDN\0\0\0\0SM\x17\0"),
            Shape::OutlookData {
                server_cache: false
            }
        );
        assert_eq!(
            of(b"!BDN\0\0\0\0SO\x24\0"),
            Shape::OutlookData { server_cache: true }
        );
        assert_eq!(of(b"!BDN\0\0\0\0XX"), Shape::Binary);
        assert_eq!(
            of(b"From someone@example.org Mon Jan  1 00:00:00 2026\nFrom: a@example.org\nSubject: hi\n\nbody\n"),
            Shape::Mbox
        );
        assert_eq!(of(b"From here to there, a story."), Shape::Text);
    }

    #[test]
    fn models_programs_caches_and_text_formats() {
        let mut g = b"GGUF".to_vec();
        g.extend_from_slice(&3u32.to_le_bytes());
        g.extend_from_slice(&[0; 16]);
        assert_eq!(of(&g), Shape::Gguf);
        let header = br#"{"t":{"dtype":"F32","shape":[1],"data_offsets":[0,4]}}"#;
        let mut st = (header.len() as u64).to_le_bytes().to_vec();
        st.extend_from_slice(header);
        st.extend_from_slice(&[0; 4]);
        assert_eq!(of(&st), Shape::Safetensors);
        let mut pe = vec![0u8; 0x100];
        pe[..2].copy_from_slice(b"MZ");
        pe[0x3c..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        pe[0x80..0x84].copy_from_slice(b"PE\0\0");
        assert_eq!(of(&pe), Shape::Executable);
        let mut not_pe = pe.clone();
        not_pe[0x80] = b'X';
        assert_ne!(of(&not_pe), Shape::Executable);
        assert_eq!(of(b"\x7fELF\x02\x01\x01"), Shape::Executable);
        assert_eq!(of(&[0xcf, 0xfa, 0xed, 0xfe, 7, 0, 0, 1]), Shape::Executable);
        assert_eq!(of(&[0xca, 0xfe, 0xba, 0xbe, 0, 0, 0, 2]), Shape::Executable);
        // A Java class file: same first bytes, a large number after them.
        assert_ne!(
            of(&[0xca, 0xfe, 0xba, 0xbe, 0, 0, 0, 0x41]),
            Shape::Executable
        );
        assert_eq!(of(b"#!/bin/sh\necho hi\n"), Shape::Executable);
        assert_eq!(
            of(b"Signature: 8a477f597d28d172789f06886806bc55\n# a cache\n"),
            Shape::CacheTag
        );
        assert_eq!(of(&SIMPLE_CACHE.to_le_bytes()), Shape::CacheEntry);
        assert_eq!(of(&0xC103_CAC3u32.to_le_bytes()), Shape::CacheEntry);
        assert_eq!(of(b"  {\"a\": 1}"), Shape::Json);
        assert_eq!(of(b"[1, 2]"), Shape::Json);
        assert_eq!(of(b"[General]\nStartWithLastProfile=1\n"), Shape::Ini);
        assert_eq!(of(b"a=1\nb=2\n[x]\nc=3\n"), Shape::Ini);
        assert_eq!(of(b"<?xml version=\"1.0\"?><config/>"), Shape::Xml);
        assert_eq!(
            of(b"<?xml version=\"1.0\"?><!DOCTYPE plist PUBLIC \"x\"><plist/>"),
            Shape::PropertyList
        );
        assert_eq!(of("\u{feff}{\"bom\": true}".as_bytes()), Shape::Json);
        assert_eq!(of(b"bplist00\xd1\x01\x02"), Shape::PropertyList);
    }
}
