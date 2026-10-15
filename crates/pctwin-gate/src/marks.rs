//! Download markers: Mark of the Web on Windows (the `Zone.Identifier` stream), the quarantine
//! flag on a Mac (the `com.apple.quarantine` attribute). The old laptop's marker is read without
//! changing anything; the new laptop's copy gets the same level, written by PCTwin in the
//! system's own form (zone and nothing else on Windows, the flag AirDrop uses on a Mac), never
//! copied byte for byte from what the old laptop sent. Linux has no standard marker.

use std::path::Path;

use cap_std::fs::Dir;

use crate::{DownloadMark, GateError};

/// The download marker the system put on the file at `path`, if any. Read only. On Windows a
/// zone of 3 is [`DownloadMark::Internet`] and 4 [`DownloadMark::Restricted`]; 0 to 2 (this
/// computer, the local network, trusted sites) is no marker; a stream that is there but cannot be
/// read is taken as [`DownloadMark::Internet`], the cautious answer. On a Mac any quarantine flag
/// is [`DownloadMark::Internet`]. On Linux always `None`.
pub fn download_mark_of(path: &Path) -> Option<DownloadMark> {
    read(path)
}

#[cfg(windows)]
fn read(path: &Path) -> Option<DownloadMark> {
    use std::io::Read;
    let mut stream = path.as_os_str().to_owned();
    stream.push(":Zone.Identifier");
    let file = match std::fs::File::open(&stream) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound || no_streams_here(&e) => return None,
        Err(_) => return Some(DownloadMark::Internet),
    };
    // A real marker is a few lines; only the start is read, however large the stream is.
    let mut text = Vec::new();
    if file.take(4096).read_to_end(&mut text).is_err() {
        return Some(DownloadMark::Internet);
    }
    let zone = String::from_utf8_lossy(&text)
        .lines()
        .find_map(|line| line.trim().strip_prefix("ZoneId=").map(str::to_owned))
        .and_then(|z| z.trim().parse::<u32>().ok());
    match zone {
        Some(0..=2) => None,
        Some(4) => Some(DownloadMark::Restricted),
        _ => Some(DownloadMark::Internet),
    }
}

#[cfg(target_os = "macos")]
fn read(path: &Path) -> Option<DownloadMark> {
    // Asking for the size only: whatever the flag says, it is there.
    match rustix::fs::getxattr(path, QUARANTINE, &mut [0u8; 0][..]) {
        Ok(_) => Some(DownloadMark::Internet),
        Err(rustix::io::Errno::RANGE) => Some(DownloadMark::Internet),
        Err(_) => None,
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
fn read(_path: &Path) -> Option<DownloadMark> {
    None
}

/// Whether `e` says this drive has no streams at all (FAT and exFAT, some network drives): then
/// no file on it carries a marker. ERROR_INVALID_FUNCTION, ERROR_NOT_SUPPORTED,
/// ERROR_INVALID_PARAMETER, ERROR_INVALID_NAME.
#[cfg(windows)]
fn no_streams_here(e: &std::io::Error) -> bool {
    matches!(e.raw_os_error(), Some(1 | 50 | 87 | 123))
}

#[cfg(target_os = "macos")]
const QUARANTINE: &str = "com.apple.quarantine";

/// Puts `mark` on the received file `file`, still under its temporary name `temp` in `dir`.
/// Returns `false` if this drive cannot hold markers at all; any other failure is an error.
#[cfg(windows)]
pub(crate) fn put(
    dir: &Dir,
    temp: &str,
    _file: &std::fs::File,
    mark: DownloadMark,
) -> Result<bool, GateError> {
    use std::io::Write;
    let zone = match mark {
        DownloadMark::Internet => 3,
        DownloadMark::Restricted => 4,
    };
    let mut options = cap_std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    let mut stream = match dir.open_with(format!("{temp}:Zone.Identifier"), &options) {
        Ok(s) => s,
        Err(e) if no_streams_here(&e) => return Ok(false),
        Err(e) => return Err(e.into()),
    };
    stream.write_all(format!("[ZoneTransfer]\r\nZoneId={zone}\r\n").as_bytes())?;
    stream.sync_all()?;
    Ok(true)
}

#[cfg(target_os = "macos")]
pub(crate) fn put(
    _dir: &Dir,
    _temp: &str,
    file: &std::fs::File,
    _mark: DownloadMark,
) -> Result<bool, GateError> {
    // The flag AirDrop gives files from another device, with when and by whom.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let value = format!("0081;{now:08x};PCTwin;");
    match rustix::fs::fsetxattr(
        file,
        QUARANTINE,
        value.as_bytes(),
        rustix::fs::XattrFlags::empty(),
    ) {
        Ok(()) => Ok(true),
        // A drive without extended attributes (some exFAT and network drives).
        Err(rustix::io::Errno::NOTSUP) => Ok(false),
        Err(e) => Err(GateError::Io(e.into())),
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
pub(crate) fn put(
    _dir: &Dir,
    _temp: &str,
    _file: &std::fs::File,
    _mark: DownloadMark,
) -> Result<bool, GateError> {
    // Linux has no standard marker: nothing to keep, nothing lost.
    Ok(true)
}
