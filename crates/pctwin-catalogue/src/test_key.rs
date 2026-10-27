//! A fixed, public test key, for tests only (feature `test-key`). Its secret is in this file, so
//! anything it signs proves nothing outside a test: [`crate::TrustedKeys::production`] never
//! holds it, and only [`keys`] or [`privileged_keys`] trust it, by name.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use blake2::{Blake2b512, Digest};
use ring::signature::{Ed25519KeyPair, KeyPair};

use crate::{Trust, TrustedKeys};

const SEED: [u8; 32] = *b"pctwin test catalogue key public";
const KEY_ID: [u8; 8] = *b"pctwtest";
const PRIVILEGED_SEED: [u8; 32] = *b"pctwin test privileged key, publ";
const PRIVILEGED_KEY_ID: [u8; 8] = *b"pctwpriv";

fn pair(seed: &[u8; 32]) -> Ed25519KeyPair {
    Ed25519KeyPair::from_seed_unchecked(seed).expect("a 32-byte seed is a key")
}

fn public_b64(seed: &[u8; 32], id: &[u8; 8]) -> String {
    let mut bin = b"Ed".to_vec();
    bin.extend_from_slice(id);
    bin.extend_from_slice(pair(seed).public_key().as_ref());
    STANDARD.encode(bin)
}

/// The test key, trusted for ordinary catalogues.
pub fn keys() -> TrustedKeys {
    TrustedKeys::from_minisign_base64(Trust::Ordinary, &[&public_b64(&SEED, &KEY_ID)])
        .expect("the test key reads")
}

/// The privileged test key, trusted for privileged catalogues (and ordinary ones).
pub fn privileged_keys() -> TrustedKeys {
    TrustedKeys::from_minisign_base64(
        Trust::Privileged,
        &[&public_b64(&PRIVILEGED_SEED, &PRIVILEGED_KEY_ID)],
    )
    .expect("the privileged test key reads")
}

fn sign_with(seed: &[u8; 32], id: &[u8; 8], bytes: &[u8], trusted_comment: &str) -> String {
    sign_form(seed, id, bytes, trusted_comment, true)
}

fn sign_form(
    seed: &[u8; 32],
    id: &[u8; 8],
    bytes: &[u8],
    trusted_comment: &str,
    prehashed: bool,
) -> String {
    let key = pair(seed);
    let sig = if prehashed {
        key.sign(&Blake2b512::digest(bytes))
    } else {
        key.sign(bytes)
    };
    let mut bin1 = if prehashed { b"ED" } else { b"Ed" }.to_vec();
    bin1.extend_from_slice(id);
    bin1.extend_from_slice(sig.as_ref());
    let mut global = sig.as_ref().to_vec();
    global.extend_from_slice(trusted_comment.as_bytes());
    let global = key.sign(&global);
    format!(
        "untrusted comment: PCTwin test key\n{}\ntrusted comment: {trusted_comment}\n{}\n",
        STANDARD.encode(bin1),
        STANDARD.encode(global.as_ref())
    )
}

/// A minisign signature of `bytes` by the test key, with exactly this trusted comment.
pub fn sign_raw(bytes: &[u8], trusted_comment: &str) -> String {
    sign_with(&SEED, &KEY_ID, bytes, trusted_comment)
}

/// A catalogue file's signature by the test key, bound to its name and version as
/// [`crate::Catalogue::signed`] requires.
pub fn sign(bytes: &[u8], catalogue: &str, version: u32) -> String {
    sign_raw(bytes, &crate::trusted_comment(catalogue, version))
}

/// The same, by the privileged test key.
pub fn sign_privileged(bytes: &[u8], catalogue: &str, version: u32) -> String {
    sign_with(
        &PRIVILEGED_SEED,
        &PRIVILEGED_KEY_ID,
        bytes,
        &crate::trusted_comment(catalogue, version),
    )
}

/// A signature in minisign's old form (the bytes signed directly, not their hash), which
/// catalogues refuse.
pub fn sign_legacy(bytes: &[u8], catalogue: &str, version: u32) -> String {
    sign_form(
        &SEED,
        &KEY_ID,
        bytes,
        &crate::trusted_comment(catalogue, version),
        false,
    )
}
