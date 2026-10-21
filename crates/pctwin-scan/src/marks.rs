//! Whether files carry a download mark (the system's note that a file came from the internet),
//! looked at on a small sample per folder and by presence only: the mark itself is never read
//! (Security Design B, "What the scan reads"). Windows keeps it in a `Zone.Identifier` stream
//! beside the file; macOS in the `com.apple.quarantine` attribute; Linux browsers in
//! `user.xdg.origin.url`.

use std::path::{Path, PathBuf};

/// How many files of a folder are looked at.
pub const SAMPLE: usize = 4;

/// What a folder's sample showed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MarkSample {
    pub looked: u32,
    pub marked: u32,
}

impl MarkSample {
    /// Every file looked at came from the internet.
    pub fn all_downloaded(&self) -> bool {
        self.looked > 0 && self.marked == self.looked
    }
}

/// Whether `path` carries a download mark. Only whether it is there is asked, never what it says.
pub fn has_download_mark(path: &Path) -> bool {
    #[cfg(windows)]
    {
        let mut stream = path.as_os_str().to_os_string();
        stream.push(":Zone.Identifier");
        std::fs::symlink_metadata(PathBuf::from(stream)).is_ok()
    }
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        let name = if cfg!(target_os = "macos") {
            "com.apple.quarantine"
        } else {
            "user.xdg.origin.url"
        };
        // An empty buffer asks only how long the attribute is: nothing of it is read.
        rustix::fs::lgetxattr(path, name, &mut [0u8; 0][..]).is_ok()
    }
    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    {
        let _ = path;
        false
    }
}

/// Looks at up to [`SAMPLE`] of `files` (a folder's own files).
pub fn sample_folder(files: impl IntoIterator<Item = PathBuf>) -> MarkSample {
    let mut s = MarkSample::default();
    for f in files.into_iter().take(SAMPLE) {
        s.looked += 1;
        if has_download_mark(&f) {
            s.marked += 1;
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_file_has_no_mark_and_a_sample_stops_at_its_size() {
        let dir = tempfile::tempdir().unwrap();
        let files: Vec<PathBuf> = (0..6)
            .map(|n| {
                let p = dir.path().join(format!("f{n}.txt"));
                std::fs::write(&p, b"x").unwrap();
                p
            })
            .collect();
        let s = sample_folder(files);
        assert_eq!(
            s,
            MarkSample {
                looked: 4,
                marked: 0
            }
        );
        assert!(!s.all_downloaded());
        assert!(!MarkSample::default().all_downloaded());
    }

    #[cfg(windows)]
    #[test]
    fn a_file_marked_as_downloaded_is_seen_without_reading_the_mark() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("setup.exe");
        std::fs::write(&p, b"x").unwrap();
        let mut stream = p.as_os_str().to_os_string();
        stream.push(":Zone.Identifier");
        std::fs::write(PathBuf::from(stream), b"[ZoneTransfer]\r\nZoneId=3\r\n").unwrap();
        assert!(has_download_mark(&p));
        assert!(sample_folder([p]).all_downloaded());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_file_marked_as_downloaded_is_seen_without_reading_the_mark() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("setup.deb");
        std::fs::write(&p, b"x").unwrap();
        // tmpfs may not take user attributes; then there is nothing to test here.
        if rustix::fs::setxattr(
            &p,
            "user.xdg.origin.url",
            b"https://example.com/setup.deb",
            rustix::fs::XattrFlags::empty(),
        )
        .is_err()
        {
            return;
        }
        assert!(has_download_mark(&p));
    }
}
