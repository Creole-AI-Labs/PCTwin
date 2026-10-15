//! Reading a received file back from the drive, past this laptop's cache, before it gets its real
//! name: "copied" then means "read back from the drive and it matched", not "the write call
//! returned".
//!
//! - A second, read-only handle on the very same file skips the system's cache: Windows
//!   `FILE_FLAG_NO_BUFFERING`, Linux `O_DIRECT`, macOS `F_NOCACHE`. macOS still answers a
//!   `F_NOCACHE` read from pages already in its cache, so there the file is also written through
//!   `F_NOCACHE`, which keeps its pages out of the cache in the first place.
//! - Reads past the cache are whole multiples of [`ALIGN`] at multiples of it, into memory aligned
//!   to it: Windows and Linux require sector-aligned offsets, lengths and buffers, and 64 KiB is a
//!   multiple of every sector size in use. The last read of a file returns fewer bytes.
//! - Each drive is tested once ([`probe`]): the uncached handle must open, read the probe file
//!   back exactly, and show it really bypasses the cache. Where it does not, the check still runs,
//!   through the cache, and says so ([`CheckedFrom::Memory`]); it is never a silent success.

use std::io::{self, Read, Write};

use cap_std::fs::{Dir, OpenOptions};

/// Reads past the cache are whole multiples of this many bytes, at offsets that are multiples of
/// it, into memory aligned to it.
pub(crate) const ALIGN: usize = 64 * 1024;
/// Most bytes asked of the drive in one read.
const MOST_AT_ONCE: usize = 4 * 1024 * 1024;
/// Size of the probe file each drive is tested with: two aligned chunks.
const PROBE_BYTES: usize = 2 * ALIGN;

/// Where the bytes of a read-back came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckedFrom {
    /// Read from the drive itself, past this laptop's cache.
    Drive,
    /// Read through this laptop's cache: the file was still checked, but its bytes may have come
    /// from this laptop's memory rather than the drive. "More details" says so.
    Memory(WhyMemory),
}

/// Why a check could not read past this laptop's cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhyMemory {
    /// The drive refused to be read past the cache (some network drives and USB sticks).
    CannotSkipCache,
    /// The drive accepted a read past the cache but did not show the read reached the drive (it
    /// allowed a read that only the cache could answer), so it is not counted as one.
    NotShown,
    /// This system has no way to read this drive past its cache.
    NoWay,
}

impl std::fmt::Display for CheckedFrom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Drive => f.write_str("checked by reading it back from the drive"),
            Self::Memory(why) => write!(
                f,
                "checked from this laptop's memory, not the drive, because {why}"
            ),
        }
    }
}

impl std::fmt::Display for WhyMemory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::CannotSkipCache => "this drive cannot be read without the system's memory cache",
            Self::NotShown => "this drive could not show that it was read directly",
            Self::NoWay => "this system cannot read this drive directly",
        })
    }
}

/// A file read back for checking, in order from its start, past the cache where
/// [`from`](Self::from) says so.
pub struct ReadBack {
    file: std::fs::File,
    from: CheckedFrom,
    /// The file's length when opened: a short read past the cache before it is an error, not
    /// the end of the file.
    len: u64,
    /// Memory for one chunk, with room to start it at an aligned address.
    store: Vec<u8>,
    /// Where the aligned chunk starts in `store`.
    start: usize,
    /// Size of one chunk: a multiple of [`ALIGN`].
    chunk: usize,
    /// Bytes of the current chunk handed out, and bytes in it.
    pos: usize,
    filled: usize,
    /// Bytes read from the file so far.
    offset: u64,
    /// The end of the file was reached.
    done: bool,
    /// For tests: the place in the file whose byte reads back flipped.
    flip_at: Option<u64>,
}

impl ReadBack {
    pub(crate) fn new(
        file: std::fs::File,
        from: CheckedFrom,
        flip_at: Option<u64>,
    ) -> io::Result<Self> {
        let len = file.metadata()?.len();
        // A small file needs a small chunk: thousands of them must not each zero megabytes.
        let want = usize::try_from(len)
            .unwrap_or(MOST_AT_ONCE)
            .clamp(1, MOST_AT_ONCE);
        let chunk = want.div_ceil(ALIGN) * ALIGN;
        let store = vec![0u8; chunk + ALIGN];
        let start = store.as_ptr().align_offset(ALIGN);
        if start >= ALIGN {
            return Err(io::Error::other("no aligned memory for reading back"));
        }
        Ok(Self {
            file,
            from,
            len,
            store,
            start,
            chunk,
            pos: 0,
            filled: 0,
            offset: 0,
            done: false,
            flip_at,
        })
    }

    /// Where its bytes come from.
    pub fn from(&self) -> CheckedFrom {
        self.from
    }

    /// Reads the next chunk from the file.
    fn fill(&mut self) -> io::Result<()> {
        let chunk = &mut self.store[self.start..self.start + self.chunk];
        let n = loop {
            match self.file.read(chunk) {
                Ok(n) => break n,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        };
        if let Some(at) = self.flip_at
            && let Some(i) = at.checked_sub(self.offset)
            && let Ok(i) = usize::try_from(i)
            && i < n
        {
            chunk[i] ^= 0xFF;
        }
        self.offset += n as u64;
        self.pos = 0;
        self.filled = n;
        let uncached = self.from == CheckedFrom::Drive;
        if n == 0 || (uncached && n % ALIGN != 0) {
            // Past the cache, a read shorter than a whole number of chunks can only be the end of
            // the file: the next read would not be aligned. Anything else is the drive failing.
            if uncached && n != 0 && self.offset < self.len {
                return Err(io::Error::other(
                    "the drive returned only part of a read before the end of the file",
                ));
            }
            self.done = true;
        }
        Ok(())
    }
}

impl Read for ReadBack {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if out.is_empty() {
            return Ok(0);
        }
        if self.pos == self.filled {
            if self.done {
                return Ok(0);
            }
            self.fill()?;
        }
        let n = out.len().min(self.filled - self.pos);
        let from = self.start + self.pos;
        out[..n].copy_from_slice(&self.store[from..from + n]);
        self.pos += n;
        Ok(n)
    }
}

/// What reading a file back block by block found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verification {
    /// Where the bytes came from: the drive, or this laptop's memory (and why).
    pub from: CheckedFrom,
    /// The blocks (numbered from 0) whose bytes did not match the sender's fingerprint, or were
    /// missing.
    pub bad_blocks: Vec<u64>,
    /// The file on the drive is longer than the file sent.
    pub too_long: bool,
}

impl Verification {
    /// Every block matched and the file is exactly as long as sent.
    pub fn passed(&self) -> bool {
        self.bad_blocks.is_empty() && !self.too_long
    }
}

/// Reads `size` bytes from `reader` block by block, each block hashed (BLAKE3) over its real
/// length (the last may be shorter), and compares each with `hashes`, the sender's fingerprints.
/// The caller checks `hashes` has one fingerprint per block.
pub(crate) fn check_blocks(
    reader: &mut ReadBack,
    size: u64,
    block_size: u64,
    hashes: &[[u8; 32]],
) -> io::Result<Verification> {
    let mut bad_blocks = Vec::new();
    let mut piece = vec![0u8; ALIGN];
    let mut ended = false;
    for (index, expected) in (0u64..).zip(hashes) {
        let start = index * block_size;
        let len = block_size.min(size - start);
        let mut hasher = blake3::Hasher::new();
        let mut left = len;
        while left > 0 && !ended {
            let want = usize::try_from(left).unwrap_or(usize::MAX).min(piece.len());
            let got = reader.read(&mut piece[..want])?;
            if got == 0 {
                ended = true;
                break;
            }
            hasher.update(&piece[..got]);
            left -= got as u64;
        }
        if left > 0 || hasher.finalize().as_bytes() != expected {
            bad_blocks.push(index);
        }
    }
    let too_long = !ended && reader.read(&mut [0u8; 1])? != 0;
    Ok(Verification {
        from: reader.from(),
        bad_blocks,
        too_long,
    })
}

/// Opens `name` in `dir` for reading past this laptop's cache.
#[cfg(windows)]
fn open_uncached_os(dir: &Dir, name: &str) -> io::Result<std::fs::File> {
    use cap_std::fs::OpenOptionsExt;
    const FILE_FLAG_NO_BUFFERING: u32 = 0x2000_0000;
    let mut options = OpenOptions::new();
    options.read(true).custom_flags(FILE_FLAG_NO_BUFFERING);
    Ok(dir.open_with(name, &options)?.into_std())
}

/// Opens `name` in `dir` for reading past this laptop's cache.
#[cfg(target_os = "linux")]
fn open_uncached_os(dir: &Dir, name: &str) -> io::Result<std::fs::File> {
    use cap_std::fs::OpenOptionsExt;
    let mut options = OpenOptions::new();
    #[allow(clippy::cast_possible_wrap)]
    options
        .read(true)
        .custom_flags(rustix::fs::OFlags::DIRECT.bits() as i32);
    Ok(dir.open_with(name, &options)?.into_std())
}

/// Opens `name` in `dir` for reading past this laptop's cache.
#[cfg(target_os = "macos")]
fn open_uncached_os(dir: &Dir, name: &str) -> io::Result<std::fs::File> {
    let file = dir.open(name)?.into_std();
    rustix::fs::fcntl_nocache(&file, true)?;
    Ok(file)
}

/// Opens `name` in `dir` for reading past this laptop's cache.
#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn open_uncached_os(_dir: &Dir, _name: &str) -> io::Result<std::fs::File> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "no way to read past the cache here",
    ))
}

/// Opens `name` in `dir` for reading past this laptop's cache, unless `refused` (for tests: a
/// drive that refuses, as some network drives and USB sticks do).
pub(crate) fn open_uncached(dir: &Dir, name: &str, refused: bool) -> io::Result<std::fs::File> {
    if refused {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "this drive refuses reads past the cache",
        ));
    }
    open_uncached_os(dir, name)
}

/// On macOS, writes `file` past the cache too: a read with `F_NOCACHE` is still answered from
/// pages already cached, and a file just written is. Whether it worked.
#[cfg(target_os = "macos")]
pub(crate) fn keep_pages_out(file: &std::fs::File) -> bool {
    rustix::fs::fcntl_nocache(file, true).is_ok()
}

/// Elsewhere a read past the cache never takes cached pages, so nothing is needed.
#[cfg(not(target_os = "macos"))]
pub(crate) fn keep_pages_out(_file: &std::fs::File) -> bool {
    true
}

/// Tests once whether files in `dir` can be read back past this laptop's cache, with a small
/// probe file written there, read back and removed.
pub(crate) fn probe(dir: &Dir, name: &str, refused: bool) -> CheckedFrom {
    let pattern: Vec<u8> = (0..PROBE_BYTES).map(|i| (i % 251) as u8 ^ 0x5A).collect();
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    let written = dir.open_with(name, &options).and_then(|file| {
        let mut file = file.into_std();
        let _ = keep_pages_out(&file);
        file.write_all(&pattern)?;
        file.sync_all()
    });
    let found = match written {
        Ok(()) => probe_file(dir, name, &pattern, refused),
        Err(_) => CheckedFrom::Memory(WhyMemory::CannotSkipCache),
    };
    let _ = dir.remove_file(name);
    found
}

fn probe_file(dir: &Dir, name: &str, pattern: &[u8], refused: bool) -> CheckedFrom {
    let Ok(file) = open_uncached(dir, name, refused) else {
        return CheckedFrom::Memory(WhyMemory::CannotSkipCache);
    };
    let mut back = Vec::new();
    match ReadBack::new(file, CheckedFrom::Drive, None).and_then(|mut r| r.read_to_end(&mut back)) {
        Ok(_) if back == pattern => {}
        _ => return CheckedFrom::Memory(WhyMemory::CannotSkipCache),
    }
    if shown_uncached(dir, name) {
        CheckedFrom::Drive
    } else {
        CheckedFrom::Memory(WhyMemory::NotShown)
    }
}

/// Whether reads of `name` in `dir` through an uncached handle really reach the drive. On Windows
/// and Linux, a read the drive itself could not do (one byte into memory at an odd address) is
/// refused only when the read bypasses the cache: a handle that accepts it is copying through
/// memory somewhere. Linux can also say so directly (`statx` direct-I/O alignment, since 6.1).
#[cfg(any(windows, target_os = "linux"))]
fn shown_uncached(dir: &Dir, name: &str) -> bool {
    let Ok(mut file) = open_uncached_os(dir, name) else {
        return false;
    };
    #[cfg(target_os = "linux")]
    if direct_io_reported(&file) {
        return true;
    }
    let mut store = vec![0u8; 2 * ALIGN];
    let odd = store.as_ptr().align_offset(ALIGN) + 1;
    matches!(file.read(&mut store[odd..=odd]), Err(e) if e.kind() == io::ErrorKind::InvalidInput)
}

/// Whether Linux reports this file can be read directly, with alignments the reader meets.
#[cfg(target_os = "linux")]
fn direct_io_reported(file: &std::fs::File) -> bool {
    use rustix::fs::{AtFlags, StatxFlags, statx};
    match statx(file, "", AtFlags::EMPTY_PATH, StatxFlags::DIOALIGN) {
        Ok(st) => {
            StatxFlags::from_bits_retain(st.stx_mask).contains(StatxFlags::DIOALIGN)
                && st.stx_dio_offset_align != 0
                && (st.stx_dio_offset_align as usize) <= ALIGN
                && st.stx_dio_mem_align != 0
                && (st.stx_dio_mem_align as usize) <= ALIGN
        }
        Err(_) => false,
    }
}

/// macOS gives no sign a `F_NOCACHE` read reached the drive. It counts as one only on a local
/// drive (`MNT_LOCAL`), where the file was also written with `F_NOCACHE` so none of its pages
/// stayed in the cache; the CI read-back job shows on a Mac that such a read-back reads the drive.
#[cfg(target_os = "macos")]
fn shown_uncached(dir: &Dir, name: &str) -> bool {
    /// `MNT_LOCAL` in `<sys/mount.h>`: the file system is on a local drive.
    const MNT_LOCAL: u32 = 0x0000_1000;
    let Ok(file) = open_uncached_os(dir, name) else {
        return false;
    };
    rustix::fs::fstatfs(&file).is_ok_and(|st| st.f_flags & MNT_LOCAL != 0)
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn shown_uncached(_dir: &Dir, _name: &str) -> bool {
    false
}
