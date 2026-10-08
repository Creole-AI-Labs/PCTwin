//! Moving one file in checked blocks (Engineering Plan section 3, Task List 1.5).
//!
//! - A file is split into blocks of [`block_size_for`] its size: 128 KiB up to 16 MiB, so a file
//!   has at most about 2,000 blocks (the Syncthing approach).
//! - [`FileSender`] reads the file once, in order. Each [`Block`] carries a BLAKE3 fingerprint of
//!   its contents and is compressed with zstd only when that shrinks it by at least 10%; formats
//!   that are already compressed are never tried.
//! - [`Block::decode`] refuses anything damaged, larger than announced, or that would expand past
//!   its announced size.
//! - [`Assembly`] writes blocks in order through the safety gate, checking each one, and gives a
//!   receipt naming the next block wanted. After a dropped connection, its [`ResumeTicket`] lets a
//!   new sender continue from exactly that block, unless the file changed in the meantime.
//! - A file that changed while it was being read is never finished; the partial file is removed
//!   and it is sent again.
//! - [`plan_order`] sets the order things move in: what the person asked for first (Smart mode or
//!   their own picks), then essentials, then the rest, newest first. [`Scheduler`] lets several
//!   files take turns piece by piece so one huge file never holds up the rest, and starts a new
//!   request at once.
//! - [`Message`] is what travels over the encrypted link; blocks go as pieces of at most
//!   [`PIECE_MAX`] and are rejoined by [`PieceBuffer`], never past the largest block.
//! - [`SenderSession`] and [`ReceiverSession`] run a whole move over a [`Channel`] (the paired
//!   link): up to 32 MiB unconfirmed at a time, every block receipted, and after a drop each file
//!   continues from exactly where it stopped on the next connection. Copies keep the original's
//!   modified time.
//! - [`landing_for`] decides where each item goes: the new laptop's own folder for its role,
//!   a "from your old laptop" folder when it has none, or left to the cloud service both laptops
//!   use for that folder.

mod landing;
mod message;
mod queue;
mod reading;
mod session;

pub use landing::{Landing, NewPlaces, approve_new_places, landing_for, role_label};
pub use message::{Message, PIECE_MAX, PieceBuffer, split_into_pieces};
pub use queue::{Scheduler, Tier, plan_order};
pub use reading::{ReadBudget, is_drive_error};
pub use session::{
    Channel, ChannelError, ReceiveOutcome, ReceiverSession, SendJob, SendOutcome, SenderSession,
};

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

use pctwin_gate::{Destination, Finished, GateError, IncomingFile, IncomingPath};

/// The smallest block.
pub const MIN_BLOCK: u64 = 128 * 1024;
/// The largest block.
pub const MAX_BLOCK: u64 = 16 * 1024 * 1024;
/// Files are split into about this many blocks at most, until blocks reach [`MAX_BLOCK`].
const MAX_BLOCKS_PER_FILE: u64 = 2000;
/// Blocks smaller than this are never compressed.
const MIN_COMPRESS: usize = 64;
/// The wire format version of a block.
const BLOCK_VERSION: u8 = 1;
const FLAG_COMPRESSED: u8 = 1;
const BLOCK_HEADER: usize = 1 + 1 + 8 + 4 + 32;

/// Why a transfer step was refused.
#[derive(Debug, thiserror::Error)]
pub enum TransferError {
    #[error("reading the file failed: {0}")]
    Io(#[from] io::Error),
    #[error("a block was damaged: {0}")]
    Damaged(String),
    #[error("expected block {expected}, got block {got}")]
    OutOfOrder { expected: u64, got: u64 },
    #[error("only {received} of {expected} blocks arrived")]
    Incomplete { received: u64, expected: u64 },
    #[error("the file changed since the transfer stopped, so it starts again")]
    ChangedSince,
    #[error("the file changed while it was being read, so it is sent again")]
    ChangedWhileRead,
    #[error(transparent)]
    Gate(#[from] GateError),
    #[error("the connection dropped; the move continues from here on the next connection")]
    ConnectionDropped,
    #[error("the other laptop sent something unexpected: {0}")]
    Protocol(String),
}

/// The block size for a file of `len` bytes: the smallest power of two from 128 KiB that keeps the
/// file to about 2,000 blocks, up to 16 MiB.
pub fn block_size_for(len: u64) -> u64 {
    let mut size = MIN_BLOCK;
    while size < MAX_BLOCK && len.div_ceil(size) > MAX_BLOCKS_PER_FILE {
        size *= 2;
    }
    size
}

/// What tells whether a file changed: its size and modified time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stamp {
    pub size: u64,
    pub modified_ns: Option<i64>,
}

impl Stamp {
    fn of(meta: &std::fs::Metadata) -> Self {
        let modified_ns =
            meta.modified()
                .ok()
                .map(|t| match t.duration_since(std::time::UNIX_EPOCH) {
                    Ok(d) => i64::try_from(d.as_nanos()).unwrap_or(i64::MAX),
                    Err(e) => -i64::try_from(e.duration().as_nanos()).unwrap_or(i64::MAX),
                });
        Self {
            size: meta.len(),
            modified_ns,
        }
    }
}

/// Sent before a file's blocks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub size: u64,
    pub block_size: u64,
    pub block_count: u64,
    /// The file as it was when reading began.
    pub stamp: Stamp,
}

/// Sent after a file's last block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trailer {
    /// The file as it was when reading ended.
    pub stamp_after: Stamp,
    changed: bool,
}

impl Trailer {
    pub(crate) fn new(stamp_after: Stamp, changed: bool) -> Self {
        Self {
            stamp_after,
            changed,
        }
    }

    pub fn changed_while_read(&self) -> bool {
        self.changed
    }
}

/// What the receiver has so far, so a new connection continues from the right block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumeTicket {
    pub next_block: u64,
    pub block_size: u64,
    pub stamp: Stamp,
}

/// The receiver's answer to each block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Receipt {
    pub next_block: u64,
}

/// One block of a file, with the fingerprint of its contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    index: u64,
    data: Vec<u8>,
    hash: [u8; 32],
    /// The compressed form, when compression helped.
    packed: Option<Vec<u8>>,
}

impl Block {
    pub fn index(&self) -> u64 {
        self.index
    }

    pub fn is_compressed(&self) -> bool {
        self.packed.is_some()
    }

    /// `[version][flags][index u64][length u32][BLAKE3 32 bytes][contents]`, contents compressed
    /// when that helped.
    pub fn encode(&self) -> Vec<u8> {
        let body = self.packed.as_deref().unwrap_or(&self.data);
        let mut out = Vec::with_capacity(BLOCK_HEADER + body.len());
        out.push(BLOCK_VERSION);
        out.push(if self.packed.is_some() {
            FLAG_COMPRESSED
        } else {
            0
        });
        out.extend_from_slice(&self.index.to_be_bytes());
        // A block is at most MAX_BLOCK (16 MiB), which always fits.
        out.extend_from_slice(
            &u32::try_from(self.data.len())
                .unwrap_or(u32::MAX)
                .to_be_bytes(),
        );
        out.extend_from_slice(&self.hash);
        out.extend_from_slice(body);
        out
    }

    /// Reads a block, refusing anything damaged, longer than `max_len`, or that would expand past
    /// its announced length.
    pub fn decode(bytes: &[u8], max_len: u64) -> Result<Self, TransferError> {
        let damaged = |why: &str| TransferError::Damaged(why.to_string());
        if bytes.len() < BLOCK_HEADER {
            return Err(damaged("too short"));
        }
        if bytes[0] != BLOCK_VERSION {
            return Err(damaged("unknown block version"));
        }
        let flags = bytes[1];
        if flags & !FLAG_COMPRESSED != 0 {
            return Err(damaged("unknown block flags"));
        }
        let index = u64::from_be_bytes(bytes[2..10].try_into().map_err(|_| damaged("index"))?);
        let len = u32::from_be_bytes(bytes[10..14].try_into().map_err(|_| damaged("length"))?);
        let len = usize::try_from(len).map_err(|_| damaged("length"))?;
        if len as u64 > max_len {
            return Err(damaged("larger than announced"));
        }
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&bytes[14..BLOCK_HEADER]);
        let body = &bytes[BLOCK_HEADER..];
        let data = if flags & FLAG_COMPRESSED != 0 {
            // The output is capped at the announced length, so it cannot expand without limit.
            zstd::bulk::decompress(body, len).map_err(|_| damaged("could not unpack"))?
        } else {
            body.to_vec()
        };
        if data.len() != len {
            return Err(damaged("wrong length"));
        }
        if blake3::hash(&data).as_bytes() != &hash {
            return Err(damaged("fingerprint does not match"));
        }
        Ok(Self {
            index,
            data,
            hash,
            packed: None,
        })
    }
}

/// A file being read on the old laptop: its bytes and its size and modified time.
pub trait Source: Read + Seek + Send {
    fn stamp(&mut self) -> io::Result<Stamp>;
}

impl Source for File {
    fn stamp(&mut self) -> io::Result<Stamp> {
        Ok(Stamp::of(&self.metadata()?))
    }
}

/// Opens files on the old laptop. [`FsOpener`] opens them directly; another opener can read from
/// elsewhere (a snapshot of files in use, or a test that makes chosen files fail).
pub trait Opener: Send + Sync {
    fn open(&self, path: &Path) -> io::Result<Box<dyn Source>>;
}

/// Opens files directly from the disk.
#[derive(Debug, Clone, Copy, Default)]
pub struct FsOpener;

impl Opener for FsOpener {
    fn open(&self, path: &Path) -> io::Result<Box<dyn Source>> {
        Ok(Box::new(File::open(path)?))
    }
}

/// Reads one file once, in order, as blocks.
pub struct FileSender {
    file: Box<dyn Source>,
    header: Header,
    next: u64,
    compressible: bool,
    changed: bool,
}

impl FileSender {
    /// Opens `path` to send, from the start or, with `resume`, from the block the receiver wants
    /// next. `compressible` is false for formats that are already compressed (photos, videos,
    /// archives), which are never tried.
    pub fn open(
        path: &Path,
        resume: Option<&ResumeTicket>,
        compressible: bool,
    ) -> Result<Self, TransferError> {
        Self::open_with(&FsOpener, path, resume, compressible)
    }

    /// As [`open`](Self::open), reading through `opener`.
    pub fn open_with(
        opener: &dyn Opener,
        path: &Path,
        resume: Option<&ResumeTicket>,
        compressible: bool,
    ) -> Result<Self, TransferError> {
        let mut file = opener.open(path)?;
        let stamp = file.stamp()?;
        let (block_size, next) = match resume {
            Some(ticket) => {
                if ticket.stamp != stamp {
                    return Err(TransferError::ChangedSince);
                }
                file.seek(SeekFrom::Start(ticket.next_block * ticket.block_size))?;
                (ticket.block_size, ticket.next_block)
            }
            None => (block_size_for(stamp.size), 0),
        };
        Ok(Self {
            file,
            header: Header {
                size: stamp.size,
                block_size,
                block_count: stamp.size.div_ceil(block_size),
                stamp,
            },
            next,
            compressible,
            changed: false,
        })
    }

    pub fn header(&self) -> &Header {
        &self.header
    }

    /// The next block, or `None` after the last one.
    pub fn next_block(&mut self) -> Result<Option<Block>, TransferError> {
        if self.next >= self.header.block_count {
            return Ok(None);
        }
        let len = block_len(&self.header, self.next);
        let mut data = vec![0u8; len];
        if let Err(e) = self.file.read_exact(&mut data) {
            if e.kind() == io::ErrorKind::UnexpectedEof {
                // The file got shorter while it was being read.
                self.changed = true;
                return Err(TransferError::ChangedWhileRead);
            }
            return Err(e.into());
        }
        let hash = *blake3::hash(&data).as_bytes();
        let packed = if self.compressible && data.len() >= MIN_COMPRESS {
            zstd::bulk::compress(&data, 1)
                .ok()
                .filter(|c| c.len() * 10 <= data.len() * 9)
        } else {
            None
        };
        let block = Block {
            index: self.next,
            data,
            hash,
            packed,
        };
        self.next += 1;
        Ok(Some(block))
    }

    /// After the last block: whether the file stayed the same while it was read.
    pub fn finish(mut self) -> Result<Trailer, TransferError> {
        let stamp_after = self.file.stamp()?;
        Ok(Trailer {
            changed: self.changed || stamp_after != self.header.stamp,
            stamp_after,
        })
    }
}

/// The length of block `index` of a file.
fn block_len(header: &Header, index: u64) -> usize {
    let start = index * header.block_size;
    let len = header.block_size.min(header.size - start);
    usize::try_from(len).unwrap_or(usize::MAX)
}

/// Receives one file's blocks, in order, through the safety gate.
pub struct Assembly<'d> {
    file: IncomingFile<'d>,
    header: Header,
    next: u64,
}

impl<'d> Assembly<'d> {
    /// Starts receiving the file described by `header` at `path` inside the approved destination.
    pub fn start(
        destination: &'d Destination,
        path: &IncomingPath,
        header: Header,
    ) -> Result<Self, TransferError> {
        if header.block_size == 0
            || header.block_size > MAX_BLOCK
            || header.block_count != header.size.div_ceil(header.block_size)
        {
            return Err(TransferError::Damaged(
                "the file's description does not add up".into(),
            ));
        }
        let mut file = destination.create_file(path, header.size)?;
        // The copy keeps the original's modified time, so a later check can tell it is unchanged.
        if let Some(ns) = header.stamp.modified_ns {
            let at = std::time::Duration::from_nanos(ns.unsigned_abs());
            let time = if ns >= 0 {
                std::time::UNIX_EPOCH.checked_add(at)
            } else {
                std::time::UNIX_EPOCH.checked_sub(at)
            };
            if let Some(time) = time {
                file.keep_modified_time(time);
            }
        }
        Ok(Self {
            file,
            header,
            next: 0,
        })
    }

    /// Checks and writes the next block; any other block is refused and changes nothing.
    pub fn accept(&mut self, block: Block) -> Result<Receipt, TransferError> {
        if block.index != self.next || self.next >= self.header.block_count {
            return Err(TransferError::OutOfOrder {
                expected: self.next,
                got: block.index,
            });
        }
        if block.data.len() != block_len(&self.header, block.index) {
            return Err(TransferError::Damaged("wrong length for its place".into()));
        }
        if blake3::hash(&block.data).as_bytes() != &block.hash {
            return Err(TransferError::Damaged("fingerprint does not match".into()));
        }
        self.file.write_all(&block.data)?;
        self.next += 1;
        Ok(Receipt {
            next_block: self.next,
        })
    }

    /// Where a new connection should continue.
    pub fn resume_ticket(&self) -> ResumeTicket {
        ResumeTicket {
            next_block: self.next,
            block_size: self.header.block_size,
            stamp: self.header.stamp,
        }
    }

    /// Gives the file its real name, only if every block arrived and the file did not change while
    /// it was read. Otherwise the partial file is removed.
    pub fn finish(self, trailer: Trailer) -> Result<Finished, TransferError> {
        if trailer.changed || trailer.stamp_after != self.header.stamp {
            return Err(TransferError::ChangedWhileRead);
        }
        if self.next != self.header.block_count {
            return Err(TransferError::Incomplete {
                received: self.next,
                expected: self.header.block_count,
            });
        }
        Ok(self.file.finish()?)
    }
}
