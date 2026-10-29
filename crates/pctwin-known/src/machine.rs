//! Telling a restored backup or a copied drive (Security Design Part T): a remembered connection
//! proves "the same device as before", and a drive copied to another laptop carries its keys, so
//! two laptops could share one identity. Each connection keeps a fingerprint of the hardware it
//! was remembered on (never the raw identifier); when this laptop's differs, the connection is
//! said to be on a restored or copied laptop, its keys are not used, and the person pairs again.
//!
//! - **Windows**: the computer's hardware identity (`Win32_ComputerSystemProduct.UUID`).
//! - **macOS**: the hardware identity (`IOPlatformUUID`).
//! - **Linux**: `/etc/machine-id`, which a copied drive carries along: weaker, and said so.
//!
//! An identity many laptops share (firmware left with all zeros, all F, or a maker's example
//! value: a catalogue, `catalogues/placeholder-machine-ids.json`) tells nothing, so it counts as
//! none: PCTwin then says it cannot tell whether this is the same laptop, never "same device".

use std::sync::OnceLock;

use sha2::{Digest, Sha256};

const LABEL: &[u8] = b"pctwin/v1/machine";
const PLACEHOLDERS: &str = include_str!("../catalogues/placeholder-machine-ids.json");

#[derive(serde::Deserialize)]
struct Placeholders {
    version: u32,
    values: Vec<String>,
}

fn placeholders() -> Option<&'static [String]> {
    static P: OnceLock<Option<Vec<String>>> = OnceLock::new();
    P.get_or_init(|| {
        let p: Placeholders = serde_json::from_str(PLACEHOLDERS).ok()?;
        (p.version == 1).then_some(p.values)
    })
    .as_deref()
}

/// `raw` as 32 upper-case hex digits (dashes, braces and spaces dropped), only when it can tell
/// this laptop from others: not one digit repeated and not a known placeholder. `None`: it
/// cannot (and a damaged catalogue trusts no identity).
pub fn usable(raw: &str) -> Option<String> {
    let digits: String = raw
        .chars()
        .filter(|c| !matches!(c, '-' | '{' | '}') && !c.is_whitespace())
        .map(|c| c.to_ascii_uppercase())
        .collect();
    if digits.len() != 32 || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let first = digits.as_bytes()[0];
    if digits.bytes().all(|b| b == first) {
        return None;
    }
    if placeholders()?.contains(&digits) {
        return None;
    }
    Some(digits)
}

/// The fingerprint of `raw` when it is [`usable`].
pub fn fingerprint(raw: &str) -> Option<String> {
    let digits = usable(raw)?;
    let mut h = Sha256::new();
    h.update(LABEL);
    h.update(digits.as_bytes());
    Some(crate::hex(&h.finalize()))
}

/// The fingerprint of this laptop's hardware identity; `None` when the system cannot say, or
/// says something many laptops share.
pub fn this_machine() -> Option<String> {
    fingerprint(&raw()?)
}

/// Whether the marker can follow a drive copied to another laptop (Linux's machine-id).
pub fn copies_follow_the_drive() -> bool {
    cfg!(not(any(windows, target_os = "macos")))
}

#[cfg(windows)]
fn raw() -> Option<String> {
    #[derive(serde::Deserialize)]
    #[expect(non_snake_case, reason = "WMI's own names")]
    struct Product {
        UUID: Option<String>,
    }
    // On a thread of its own, so the caller's COM setup never matters.
    std::thread::spawn(|| {
        let con = wmi::WMIConnection::new().ok()?;
        let rows: Vec<Product> = con
            .raw_query("SELECT UUID FROM Win32_ComputerSystemProduct")
            .ok()?;
        rows.into_iter().find_map(|p| p.UUID)
    })
    .join()
    .ok()
    .flatten()
}

#[cfg(target_os = "macos")]
fn raw() -> Option<String> {
    let out = std::process::Command::new("/usr/sbin/ioreg")
        .args(["-rd1", "-c", "IOPlatformExpertDevice"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .find(|l| l.contains("\"IOPlatformUUID\""))
        .and_then(|l| l.rsplit('"').nth(1))
        .map(str::to_string)
}

#[cfg(not(any(windows, target_os = "macos")))]
fn raw() -> Option<String> {
    std::fs::read_to_string("/etc/machine-id")
        .or_else(|_| std::fs::read_to_string("/var/lib/dbus/machine-id"))
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_placeholder_catalogue_reads() {
        let p = placeholders().unwrap();
        assert!(!p.is_empty());
        for v in p {
            assert_eq!(v.len(), 32, "{v}");
            assert!(
                v.bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_lowercase())
            );
        }
    }
}
