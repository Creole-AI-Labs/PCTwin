//! Saving PCTwin's own records (the before-wipe list, the recovery kit): the one place this crate
//! writes. A record is written to a fresh temporary file beside its place, flushed to disk, then
//! moved into place, so a crash leaves the earlier save or this one, never a mix.

use std::path::Path;

use serde::Serialize;
use serde::de::DeserializeOwned;

/// Most bytes a saved record may have when read back.
pub const MAX_RECORD: u64 = 256 * 1024 * 1024;

fn folder_of(path: &Path) -> &Path {
    match path.parent() {
        Some(d) if !d.as_os_str().is_empty() => d,
        _ => Path::new("."),
    }
}

fn temp_in(dir: &Path) -> std::io::Result<tempfile::NamedTempFile> {
    tempfile::Builder::new()
        .prefix(".pctwin-save.")
        .suffix(".tmp")
        .tempfile_in(dir)
}

/// Saves `value` as JSON at `path` (PCTwin's own file there is replaced as a whole).
pub fn save<T: Serialize>(path: &Path, value: &T) -> std::io::Result<()> {
    let mut temp = temp_in(folder_of(path))?;
    serde_json::to_writer_pretty(std::io::BufWriter::new(&mut temp), value)
        .map_err(std::io::Error::other)?;
    temp.as_file().sync_all()?;
    temp.persist(path).map_err(|e| e.error)?;
    Ok(())
}

/// Reads JSON saved by [`save`].
pub fn load<T: DeserializeOwned>(path: &Path) -> std::io::Result<T> {
    let file = std::fs::File::open(path)?;
    if file.metadata()?.len() > MAX_RECORD {
        return Err(std::io::Error::other("the saved record is too large"));
    }
    serde_json::from_reader(std::io::BufReader::new(file)).map_err(std::io::Error::other)
}
