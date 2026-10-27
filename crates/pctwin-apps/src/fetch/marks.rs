//! Keeping a download's origin on it, as the system expects of a browser (the 2.1 plan, section
//! 9.5): never removed to quiet a warning.
//!
//! - **Windows:** the mark of the web, an NTFS stream named `Zone.Identifier` beside the file:
//!   `[ZoneTransfer]`, `ZoneId=3` (the Internet zone) and `HostUrl=` the address it came from,
//!   without its query (which can carry a private token, T21-051).
//! - **Mac:** the `com.apple.quarantine` flag, set by the app on the file it reads back (the
//!   fetch process's sandbox writes only the file).
//! - **Linux:** no quarantine exists; the origin is written as `user.xdg.origin.url` where the
//!   file system allows it.

use std::path::Path;

/// The mark-of-the-web text for a download from `from` (already without its query).
pub fn zone_identifier(from: &str) -> String {
    let from: String = from.chars().filter(|c| !c.is_control()).collect();
    format!("[ZoneTransfer]\r\nZoneId=3\r\nHostUrl={from}\r\n")
}

/// Marks `file` as downloaded from `from`.
pub fn mark(file: &Path, from: &str) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        let mut stream = file.as_os_str().to_owned();
        stream.push(":Zone.Identifier");
        std::fs::write(stream, zone_identifier(from))
    }
    #[cfg(not(windows))]
    {
        // The Mac's quarantine flag and Linux's origin are set by the app once the file is back
        // (see `process`); nothing to write from inside the sandbox.
        let _ = (file, from);
        Ok(())
    }
}

/// Whether `file` still carries its mark (Windows); elsewhere always true here.
pub fn marked(file: &Path) -> bool {
    #[cfg(windows)]
    {
        let mut stream = file.as_os_str().to_owned();
        stream.push(":Zone.Identifier");
        std::fs::read_to_string(stream).is_ok_and(|t| t.contains("ZoneId=3"))
    }
    #[cfg(not(windows))]
    {
        let _ = file;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mark_names_the_internet_zone_and_the_origin_without_secrets() {
        let t = zone_identifier("https://dl.example.com/setup.exe\r\nZoneId=0");
        assert!(t.starts_with("[ZoneTransfer]\r\nZoneId=3\r\n"));
        // A line break in the address cannot add a line of its own.
        assert_eq!(t.matches("ZoneId=").count(), 2);
        assert_eq!(t.lines().filter(|l| l.starts_with("ZoneId=")).count(), 1);
    }

    #[cfg(windows)]
    #[test]
    fn a_download_keeps_its_mark_of_the_web() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("setup.exe");
        std::fs::write(&f, b"MZ").unwrap();
        assert!(!marked(&f));
        mark(&f, "https://dl.example.com/setup.exe").unwrap();
        assert!(marked(&f));
        // The file's own bytes are untouched.
        assert_eq!(std::fs::read(&f).unwrap(), b"MZ");
    }
}
