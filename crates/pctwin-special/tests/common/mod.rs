//! Synthetic app data, made fresh in a temporary folder for each test. Every byte is invented
//! here: no real profile, mail, key or save is ever read or copied. Folder names imitate how some
//! apps lay out their data only so the tests show that names do not matter; the code under test
//! never sees an app's name.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use sha2::Digest;

pub fn put(p: &Path, bytes: &[u8]) {
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, bytes).unwrap();
}

/// An SQLite database made by SQLite itself, closed (no log left beside it).
pub fn sqlite(p: &Path, sql: &str) {
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    let c = rusqlite::Connection::open(p).unwrap();
    c.execute_batch(sql).unwrap();
}

/// A value in the PKCS#11 secret-decoder-ring form (key ID, cipher parameters, ciphertext), with
/// invented bytes.
pub fn sdr_value() -> String {
    let mut inner = vec![0x04, 0x10];
    inner.extend_from_slice(&[0xf8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
    inner.extend_from_slice(&[0x30, 0x05, 0x06, 0x03, 0x2a, 0x86, 0x48]);
    inner.extend_from_slice(&[0x04, 0x08, 9, 9, 9, 9, 9, 9, 9, 9]);
    let mut b = vec![0x30, inner.len() as u8];
    b.extend_from_slice(&inner);
    STANDARD.encode(b)
}

/// An OpenSSH private key file with invented contents: `cipher` "none" (no passphrase) or a
/// cipher name; `key_type` such as "ssh-ed25519" or "sk-ssh-ed25519@openssh.com".
pub fn openssh_key(cipher: &str, key_type: &str) -> Vec<u8> {
    let mut b = b"openssh-key-v1\0".to_vec();
    for s in [cipher.as_bytes(), b"none", b""] {
        b.extend_from_slice(&(s.len() as u32).to_be_bytes());
        b.extend_from_slice(s);
    }
    b.extend_from_slice(&1u32.to_be_bytes());
    let mut key = (key_type.len() as u32).to_be_bytes().to_vec();
    key.extend_from_slice(key_type.as_bytes());
    key.extend_from_slice(&[0, 0, 0, 4, 1, 2, 3, 4]);
    b.extend_from_slice(&(key.len() as u32).to_be_bytes());
    b.extend_from_slice(&key);
    format!(
        "-----BEGIN OPENSSH PRIVATE KEY-----\n{}\n-----END OPENSSH PRIVATE KEY-----\n",
        STANDARD.encode(b)
    )
    .into_bytes()
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    sha2::Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// A GGUF header that says it is part `no` (from 0) of `count`.
pub fn gguf_part(no: u16, count: u16) -> Vec<u8> {
    let mut b = b"GGUF".to_vec();
    b.extend_from_slice(&3u32.to_le_bytes());
    b.extend_from_slice(&0u64.to_le_bytes());
    b.extend_from_slice(&2u64.to_le_bytes());
    for (k, v) in [("split.no", no), ("split.count", count)] {
        b.extend_from_slice(&(k.len() as u64).to_le_bytes());
        b.extend_from_slice(k.as_bytes());
        b.extend_from_slice(&2u32.to_le_bytes());
        b.extend_from_slice(&v.to_le_bytes());
    }
    b.extend_from_slice(&[0u8; 64]);
    b
}

/// A mail-program profile laid out the way profile-based mail programs do: an INI file listing
/// profiles, local folders in mbox, an address book database, saved passwords under a key store.
pub fn mail_profile(root: &Path) {
    put(
        &root.join("profiles.ini"),
        b"[General]\nStartWithLastProfile=1\n\n[Profile0]\nName=default\nIsRelative=1\nPath=Profiles/abc.default\n",
    );
    let p = root.join("Profiles/abc.default");
    put(
        &p.join("Mail/Local Folders/Inbox"),
        b"From someone@example.org Mon Jan  5 10:00:00 2026\nFrom: someone@example.org\nSubject: synthetic\n\nhello\n",
    );
    sqlite(
        &p.join("abook.sqlite"),
        "CREATE TABLE cards(uid TEXT, name TEXT); INSERT INTO cards VALUES('1','A Person');",
    );
    sqlite(
        &p.join("global-messages-db.sqlite"),
        "CREATE TABLE messages(id INTEGER, subject TEXT);",
    );
    key_store(&p);
    put(
        &p.join("prefs.js"),
        b"user_pref(\"mail.synthetic\", true);\n",
    );
    put(&p.join("cache2/entries/0A1B"), b"cached bytes");
}

/// A key database and the saved-passwords file encrypted under it (invented values).
pub fn key_store(dir: &Path) {
    sqlite(
        &dir.join("key4.db"),
        "CREATE TABLE metaData(id TEXT, item1 BLOB); CREATE TABLE nssPrivate(id INTEGER, a11 BLOB);",
    );
    let v = sdr_value();
    put(
        &dir.join("logins.json"),
        format!(r#"{{"logins":[{{"hostname":"https://example.org","encryptedUsername":"{v}","encryptedPassword":"{v}"}}]}}"#)
            .as_bytes(),
    );
}

/// A browser profile of the kind NSS-based browsers keep.
pub fn nss_browser_profile(root: &Path) {
    put(
        &root.join("profiles.ini"),
        b"[Profile0]\nName=default\nIsRelative=1\nPath=Profiles/xyz.default\n",
    );
    let p = root.join("Profiles/xyz.default");
    sqlite(
        &p.join("places.sqlite"),
        "CREATE TABLE moz_places(id INTEGER, url TEXT, title TEXT);",
    );
    sqlite(
        &p.join("cookies.sqlite"),
        "CREATE TABLE moz_cookies(id INTEGER, host TEXT, name TEXT, value TEXT, path TEXT, expiry INTEGER);",
    );
    key_store(&p);
    put(&p.join("cache2/doomed/x"), b"cache");
}

/// A browser profile of the kind Chromium-based browsers keep.
pub fn chromium_profile(root: &Path) {
    let key = STANDARD.encode(b"DPAPI\x01\x00\x00\x00 invented wrapped key bytes");
    put(
        &root.join("Local State"),
        format!(r#"{{"os_crypt":{{"encrypted_key":"{key}"}},"browser":{{"x":1}}}}"#).as_bytes(),
    );
    let d = root.join("Default");
    put(
        &d.join("Bookmarks"),
        br#"{"roots":{"bookmark_bar":{"children":[{"name":"Example","url":"https://example.org"}]}},"version":1}"#,
    );
    sqlite(
        &d.join("History"),
        "CREATE TABLE urls(id INTEGER, url TEXT, title TEXT, visit_count INTEGER);",
    );
    sqlite(
        &d.join("Cookies"),
        "CREATE TABLE cookies(creation_utc INTEGER, host_key TEXT, name TEXT, encrypted_value BLOB, path TEXT, expires_utc INTEGER);",
    );
    sqlite(
        &d.join("Login Data"),
        "CREATE TABLE logins(origin_url TEXT, username_value TEXT, password_value BLOB);",
    );
    sqlite(
        &d.join("Web Data"),
        "CREATE TABLE credit_cards(guid TEXT, name_on_card TEXT, card_number_encrypted BLOB);",
    );
    put(
        &d.join("Local Storage/leveldb/CURRENT"),
        b"MANIFEST-000001\n",
    );
    put(&d.join("Local Storage/leveldb/MANIFEST-000001"), b"x");
    put(&d.join("Cache/Cache_Data/index"), b"x");
    put(&d.join("Code Cache/js/index"), b"x");
    let mut pe = vec![0u8; 0x100];
    pe[..2].copy_from_slice(b"MZ");
    pe[0x3c..0x40].copy_from_slice(&0x80u32.to_le_bytes());
    pe[0x80..0x84].copy_from_slice(b"PE\0\0");
    put(&d.join("Extensions/abc/1.0/native.dll"), &pe);
}

/// A password manager's database and its XML key file (invented bytes).
pub fn password_vault(root: &Path) {
    let mut k = vec![0x03, 0xd9, 0xa2, 0x9a, 0x67, 0xfb, 0x4b, 0xb5, 0, 0, 4, 0];
    k.extend_from_slice(&[7; 64]);
    put(&root.join("Vault.kdbx"), &k);
    put(
        &root.join("unlock.keyx"),
        b"<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<KeyFile><Meta><Version>2.0</Version></Meta><Key><Data>AAAA</Data></Key></KeyFile>\n",
    );
    put(
        &root.join("Settings.xml"),
        b"<?xml version=\"1.0\"?><Config><Lang>en</Lang></Config>",
    );
}

/// A home's SSH folder with invented keys.
pub fn ssh_folder(root: &Path) {
    put(
        &root.join("id_ed25519"),
        &openssh_key("none", "ssh-ed25519"),
    );
    put(
        &root.join("id_ed25519.pub"),
        b"ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAISyntheticKey person@laptop\n",
    );
    put(
        &root.join("work_key"),
        &openssh_key("aes256-ctr", "ssh-ed25519"),
    );
    put(
        &root.join("id_ed25519_sk"),
        &openssh_key("none", "sk-ssh-ed25519@openssh.com"),
    );
    put(
        &root.join("known_hosts"),
        b"example.org ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAISyntheticHost\n",
    );
    put(
        &root.join("config"),
        b"Host example\n  HostName example.org\n  User person\n",
    );
}

/// A game's save folder.
pub fn game_saves(root: &Path) {
    put(
        &root.join("Profile 1/slot1.sav"),
        &[0x53, 0x41, 0x56, 0, 1, 2, 3, 0xff],
    );
    put(&root.join("settings.ini"), b"[Video]\nWidth=1920\n");
}

/// A local model library: blobs named by their own SHA-256 and a manifest listing them.
/// `missing_one`: the manifest also lists a blob that is not there. Returns the blobs' hashes.
pub fn model_library(root: &Path, missing_one: bool) -> Vec<String> {
    let weights = b"GGUF\x03\x00\x00\x00 invented weights".to_vec();
    let config = br#"{"model_format":"gguf"}"#.to_vec();
    let mut hashes = Vec::new();
    for bytes in [&weights, &config] {
        let h = sha256_hex(bytes);
        put(&root.join(format!("models/blobs/sha256-{h}")), bytes);
        hashes.push(h);
    }
    let mut layers: Vec<String> = hashes
        .iter()
        .map(|h| format!(r#"{{"digest":"sha256:{h}"}}"#))
        .collect();
    if missing_one {
        layers.push(format!(r#"{{"digest":"sha256:{}"}}"#, "e".repeat(64)));
    }
    put(
        &root.join("models/manifests/registry.example/library/m/latest"),
        format!(r#"{{"schemaVersion":2,"layers":[{}]}}"#, layers.join(",")).as_bytes(),
    );
    hashes
}

pub fn temp() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

pub fn home_of(t: &tempfile::TempDir) -> PathBuf {
    t.path().join("home")
}
