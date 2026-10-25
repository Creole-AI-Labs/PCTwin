//! Other people in one move (Security Design part C, "The helper in Release 1"): their files,
//! found and read by the administrator helper, join the move. Each one is planned under a source
//! that names its helper handle, never a path, and is read through the helper by
//! [`HelperOpener`]; everything else is opened from the disk as always. Their names travel in the
//! move record and are never shown to the mover (decided 21 October 2026).

use std::collections::HashMap;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use pctwin_helper::protocol::MAX_READ;
use pctwin_helper::{Client, ClientError, Refusal, Scanned};
use pctwin_record::{ItemId, Record};
use pctwin_transfer::{FsOpener, InUse, OnlineOnly, Opener, Source, Stamp};

/// The start of a source that names a helper handle. It is not a path any system makes (a
/// relative name with a colon, which Windows refuses and no scan produces), and only the
/// [`HelperOpener`] reads it.
pub const HELPER_SOURCE: &str = "pctwin-helper-file:";

/// The source naming `handle`.
pub fn helper_source(handle: u64) -> PathBuf {
    PathBuf::from(format!("{HELPER_SOURCE}{handle}"))
}

fn handle_of(path: &Path) -> Option<u64> {
    path.to_str()?.strip_prefix(HELPER_SOURCE)?.parse().ok()
}

/// Adds the people the helper scanned to `record`, and returns each new item's helper handle
/// (for [`helper_source`]). An item already in the record (the same ID) is not added twice.
pub fn add_people(record: &mut Record, scanned: &[Scanned]) -> HashMap<ItemId, u64> {
    let mut handles = HashMap::new();
    let mut known: std::collections::HashSet<ItemId> = record.items.iter().map(|i| i.id).collect();
    for person in scanned {
        for (item, handle) in &person.items {
            if known.insert(item.id) {
                record.items.push(item.clone());
                handles.insert(item.id, handle.handle);
            }
        }
    }
    handles
}

/// Opens a move's files: those named by [`helper_source`] through the helper session, every
/// other one from the disk ([`FsOpener`]). `files` gives each handle's size and changed time as
/// the helper's scan saw them.
pub struct HelperOpener<S: Read + Write + Send> {
    client: Arc<Mutex<Client<S>>>,
    files: HashMap<u64, Stamp>,
}

impl<S: Read + Write + Send> HelperOpener<S> {
    pub fn new(client: Arc<Mutex<Client<S>>>, scanned: &[Scanned]) -> Self {
        let files = scanned
            .iter()
            .flat_map(|p| &p.items)
            .map(|(item, h)| {
                (
                    h.handle,
                    Stamp {
                        size: h.size,
                        modified_ns: item.modified_ns,
                    },
                )
            })
            .collect();
        Self { client, files }
    }
}

impl<S: Read + Write + Send + 'static> Opener for HelperOpener<S> {
    fn open(&self, path: &Path) -> io::Result<Box<dyn Source>> {
        let Some(handle) = handle_of(path) else {
            return FsOpener.open(path);
        };
        let stamp = *self
            .files
            .get(&handle)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no such helper file"))?;
        Ok(Box::new(HelperFile {
            client: Arc::clone(&self.client),
            handle,
            stamp,
            pos: 0,
        }))
    }
}

/// One file read through the helper.
struct HelperFile<S: Read + Write + Send> {
    client: Arc<Mutex<Client<S>>>,
    handle: u64,
    stamp: Stamp,
    pos: u64,
}

/// The helper's refusal as the error the transfer understands: online only is said as such, a
/// file gone as not found (so it is "no longer there"); anything else is a read that failed.
fn as_io(e: ClientError) -> io::Error {
    match e {
        ClientError::Refused(Refusal::OnlineOnly) => io::Error::other(OnlineOnly),
        ClientError::Refused(Refusal::InUse) => io::Error::other(InUse),
        ClientError::Refused(Refusal::Gone) => {
            io::Error::new(io::ErrorKind::NotFound, Refusal::Gone.to_string())
        }
        other => io::Error::other(other.to_string()),
    }
}

impl<S: Read + Write + Send> Read for HelperFile<S> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let left = self.stamp.size.saturating_sub(self.pos);
        let want = (buf.len() as u64).min(left).min(u64::from(MAX_READ));
        if want == 0 {
            return Ok(0);
        }
        let bytes = self
            .client
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .read(self.handle, self.pos, want as u32)
            .map_err(as_io)?;
        buf[..bytes.len()].copy_from_slice(&bytes);
        self.pos += bytes.len() as u64;
        Ok(bytes.len())
    }
}

impl<S: Read + Write + Send> Seek for HelperFile<S> {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        let new = match to {
            SeekFrom::Start(n) => Some(n),
            SeekFrom::End(d) => self.stamp.size.checked_add_signed(d),
            SeekFrom::Current(d) => self.pos.checked_add_signed(d),
        }
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "a seek before the start"))?;
        self.pos = new;
        Ok(new)
    }
}

impl<S: Read + Write + Send> Source for HelperFile<S> {
    fn stamp(&mut self) -> io::Result<Stamp> {
        Ok(self.stamp)
    }
}
