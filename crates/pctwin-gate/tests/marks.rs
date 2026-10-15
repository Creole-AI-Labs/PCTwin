//! Download markers (Security Design B, "Nothing received runs on its own"): a file the old
//! laptop's system marked as from the internet (Mark of the Web on Windows, the quarantine flag
//! on a Mac) arrives marked too, so the new laptop's system stays just as cautious with it.
//! Only the marker's level travels (internet or restricted), never the web address or anything
//! else the old laptop wrote. Linux has no standard marker, so there is nothing to keep there.

use std::io::Write;
use std::path::Path;

use pctwin_gate::{
    Destination, DownloadMark, DriveKind, DriveProfile, IncomingPath, download_mark_of,
};

fn receive(dest: &Destination, name: &str, mark: Option<DownloadMark>) -> String {
    let mut file = dest
        .create_file(&IncomingPath::parse(name).unwrap(), 3)
        .unwrap();
    file.write_all(b"abc").unwrap();
    if let Some(mark) = mark {
        file.keep_download_mark(mark);
    }
    let when = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_600_000_000);
    file.keep_modified_time(when);
    file.finish().unwrap().final_path
}

/// What the new laptop's system now says about the file at `path`.
fn mark_on(path: &Path) -> Option<DownloadMark> {
    download_mark_of(path)
}

#[test]
fn a_marked_file_arrives_marked_and_keeps_its_time() {
    let root = tempfile::tempdir().unwrap();
    let dest = Destination::open(root.path()).unwrap();
    let name = receive(&dest, "Downloads/setup.exe", Some(DownloadMark::Internet));
    let landed = root.path().join(&name);
    assert_eq!(std::fs::read(&landed).unwrap(), b"abc");
    if cfg!(any(windows, target_os = "macos")) {
        assert_eq!(mark_on(&landed), Some(DownloadMark::Internet));
    } else {
        assert_eq!(mark_on(&landed), None, "Linux has no standard marker");
    }
    // Writing the marker does not change the modified time the file keeps.
    let modified = std::fs::metadata(&landed).unwrap().modified().unwrap();
    assert_eq!(
        modified,
        std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_600_000_000)
    );
}

#[test]
fn an_unmarked_file_arrives_unmarked() {
    let root = tempfile::tempdir().unwrap();
    let dest = Destination::open(root.path()).unwrap();
    let name = receive(&dest, "notes.txt", None);
    assert_eq!(mark_on(&root.path().join(name)), None);
}

#[test]
fn a_clashing_name_still_carries_the_mark() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("setup.exe"), b"mine").unwrap();
    let dest = Destination::open(root.path()).unwrap();
    let name = receive(&dest, "setup.exe", Some(DownloadMark::Internet));
    assert_eq!(name, "setup (2).exe");
    if cfg!(any(windows, target_os = "macos")) {
        assert_eq!(
            mark_on(&root.path().join(&name)),
            Some(DownloadMark::Internet)
        );
    }
    // The person's own file is untouched, marker and all.
    assert_eq!(mark_on(&root.path().join("setup.exe")), None);
}

#[cfg(windows)]
mod windows {
    use super::*;

    fn stream(path: &Path) -> std::path::PathBuf {
        let mut s = path.as_os_str().to_owned();
        s.push(":Zone.Identifier");
        s.into()
    }

    #[test]
    fn the_marker_is_the_standard_zone_stream() {
        let root = tempfile::tempdir().unwrap();
        let dest = Destination::open(root.path()).unwrap();
        let name = receive(&dest, "a.exe", Some(DownloadMark::Restricted));
        let text = std::fs::read_to_string(stream(&root.path().join(name))).unwrap();
        assert_eq!(text, "[ZoneTransfer]\r\nZoneId=4\r\n");
    }

    #[test]
    fn the_old_laptops_marker_is_read_by_its_zone() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("f.bin");
        std::fs::write(&file, b"x").unwrap();
        assert_eq!(download_mark_of(&file), None, "no stream");
        for (text, want) in [
            (
                "[ZoneTransfer]\r\nZoneId=3\r\nHostUrl=https://example.com/x\r\n",
                Some(DownloadMark::Internet),
            ),
            ("[ZoneTransfer]\nZoneId=4\n", Some(DownloadMark::Restricted)),
            ("[ZoneTransfer]\r\nZoneId=2\r\n", None),
            ("[ZoneTransfer]\r\nZoneId=0\r\n", None),
            // Present but unreadable: the cautious answer.
            ("rubbish", Some(DownloadMark::Internet)),
            (
                "[ZoneTransfer]\r\nZoneId=99\r\n",
                Some(DownloadMark::Internet),
            ),
        ] {
            std::fs::write(stream(&file), text).unwrap();
            assert_eq!(download_mark_of(&file), want, "{text:?}");
        }
        // A stream that is there but cannot be opened (another program holds it): marked.
        {
            use std::os::windows::fs::OpenOptionsExt;
            let _held = std::fs::OpenOptions::new()
                .read(true)
                .share_mode(0)
                .open(stream(&file))
                .unwrap();
            assert_eq!(download_mark_of(&file), Some(DownloadMark::Internet));
        }
        // A huge stream is not read whole, and still counts as marked.
        std::fs::write(stream(&file), vec![b'x'; 1_000_000]).unwrap();
        assert_eq!(download_mark_of(&file), Some(DownloadMark::Internet));
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::*;

    #[test]
    fn the_marker_is_the_quarantine_attribute() {
        let root = tempfile::tempdir().unwrap();
        let dest = Destination::open(root.path()).unwrap();
        let name = receive(&dest, "a.dmg", Some(DownloadMark::Internet));
        let mut value = [0u8; 256];
        let n = rustix::fs::getxattr(root.path().join(name), "com.apple.quarantine", &mut value)
            .unwrap();
        let text = std::str::from_utf8(&value[..n]).unwrap();
        assert!(text.starts_with("0081;"), "{text}");
        assert!(text.ends_with(";PCTwin;"), "{text}");
    }

    #[test]
    fn the_old_laptops_quarantine_flag_is_read() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("f.bin");
        std::fs::write(&file, b"x").unwrap();
        assert_eq!(download_mark_of(&file), None);
        rustix::fs::setxattr(
            &file,
            "com.apple.quarantine",
            b"0083;62e09bd1;Safari;",
            rustix::fs::XattrFlags::empty(),
        )
        .unwrap();
        assert_eq!(download_mark_of(&file), Some(DownloadMark::Internet));
    }
}

/// On a drive that cannot hold download markers (FAT and exFAT, some network drives), the file
/// still lands, as Windows' own copying does, but never silently: the finished file says which
/// marker could not be kept, so the report can tell the person plainly.
#[test]
fn a_drive_that_cannot_keep_markers_still_gets_the_file_and_says_so() {
    let root = tempfile::tempdir().unwrap();
    let dest = Destination::open(root.path())
        .unwrap()
        .with_profile(DriveProfile::of(DriveKind::Fat).without_markers());
    let mut file = dest
        .create_file(&IncomingPath::parse("setup.exe").unwrap(), 3)
        .unwrap();
    file.write_all(b"abc").unwrap();
    file.keep_download_mark(DownloadMark::Restricted);
    let done = file.finish().unwrap();
    assert_eq!(done.mark_not_kept, Some(DownloadMark::Restricted));
    assert_eq!(
        std::fs::read(root.path().join("setup.exe")).unwrap(),
        b"abc"
    );
    assert_eq!(download_mark_of(&root.path().join("setup.exe")), None);

    // An unmarked file loses nothing.
    let mut file = dest
        .create_file(&IncomingPath::parse("notes.txt").unwrap(), 1)
        .unwrap();
    file.write_all(b"x").unwrap();
    assert_eq!(file.finish().unwrap().mark_not_kept, None);
}

/// Where the marker is kept, nothing is reported lost.
#[test]
fn a_kept_marker_is_not_reported_lost() {
    let root = tempfile::tempdir().unwrap();
    let dest = Destination::open(root.path()).unwrap();
    let mut file = dest
        .create_file(&IncomingPath::parse("a.exe").unwrap(), 1)
        .unwrap();
    file.write_all(b"x").unwrap();
    file.keep_download_mark(DownloadMark::Internet);
    assert_eq!(file.finish().unwrap().mark_not_kept, None);
}

/// Restricted is stricter than internet, and any marker stricter than none: when two answers
/// differ (after a restart), the stricter is kept.
#[test]
fn markers_are_ordered_from_least_to_most_cautious() {
    assert!(Some(DownloadMark::Restricted) > Some(DownloadMark::Internet));
    assert!(Some(DownloadMark::Internet) > None);
}
