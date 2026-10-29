//! Telling a restored backup or a copied drive (Security Design Part T): a remembered connection
//! proves "the same device as before", and a drive copied to another laptop carries its keys, so
//! two laptops could share one identity. Each connection keeps a fingerprint of the hardware it
//! was remembered on (never the raw identifier); when this laptop's differs, the connection is
//! said to be on a restored or copied laptop, its keys are not used, and the person pairs again.
//!
//! - **Windows**: the computer's hardware identity (`Win32_ComputerSystemProduct.UUID`).
//! - **macOS**: the hardware identity (`IOPlatformUUID`).
//! - **Linux**: `/etc/machine-id`, which a copied drive carries along: weaker, and said so.

use sha2::{Digest, Sha256};

const LABEL: &[u8] = b"pctwin/v1/machine";

/// The fingerprint of this laptop's hardware identity; `None` when the system cannot say.
pub fn this_machine() -> Option<String> {
    let raw = raw()?;
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    let mut h = Sha256::new();
    h.update(LABEL);
    h.update(raw.as_bytes());
    Some(crate::hex(&h.finalize()))
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
