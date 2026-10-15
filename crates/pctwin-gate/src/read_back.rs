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
//! - Then each file is judged on its own ([`this_file_from_drive`]): a system can quietly read a
//!   file through its memory even on a drive that passed, because of how that file is stored
//!   (Windows: compressed, encrypted, or so small it lives in the drive's own records; Linux:
//!   kept inline, journalled, compressed, encrypted or verity-protected; ZFS, whose direct reads
//!   may still come from its own cache). Such a file is read through an ordinary handle, still
//!   checked, and its result says it came from memory. The file is judged again after the last
//!   read, so one changed while it was read (another program compressing it) says memory too.
//!   "Drive" is said only when it is known for the whole read.
//! - An input or output error while reading back fails the check with that error (the file is
//!   removed, never named), rather than as blocks that did not match.

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
    /// The file is stored in a way the system reads through its memory (compressed, encrypted,
    /// kept inline or journalled), whatever the drive allows.
    StoredSpecially,
    /// The file is so small the drive may keep it inside its own records, which the system reads
    /// through its memory.
    TooSmall,
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
            Self::StoredSpecially => {
                "this file is stored compressed or protected, which the system reads through memory"
            }
            Self::TooSmall => "this file is too small to be sure the drive itself was read",
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

    /// After the last read: judges the file again with `judge`, so a file that changed how it is
    /// stored while it was read (compressed by another program, say) no longer counts as read
    /// from the drive. Returns where its bytes came from, for the whole read.
    pub(crate) fn judged_after(
        &mut self,
        judge: impl FnOnce(&std::fs::File) -> Result<(), WhyMemory>,
    ) -> CheckedFrom {
        if self.from == CheckedFrom::Drive
            && let Err(why) = judge(&self.file)
        {
            self.from = CheckedFrom::Memory(why);
        }
        self.from
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

/// Files smaller than this may be kept inside the drive's own records on Windows (NTFS keeps small
/// files in the master file table, up to most of a record, and records can be 4 KiB), and are
/// then read through the system's memory whatever the handle asks.
#[cfg(windows)]
const SMALLEST_FROM_DRIVE: u64 = 4096;

/// Whether this very file, opened past the cache on a drive that passed [`probe`], is really read
/// from the drive; else why not. Decided from the file's own attributes, never assumed.
#[cfg(windows)]
pub(crate) fn this_file_from_drive(file: &std::fs::File) -> Result<(), WhyMemory> {
    use std::os::windows::fs::MetadataExt;
    /// Sparse, compressed, offline, encrypted, and the two cloud "recall" attributes: each makes
    /// the system read the file through its memory, or from somewhere else.
    const READ_THROUGH_MEMORY: u32 =
        0x0000_0200 | 0x0000_0800 | 0x0000_1000 | 0x0000_4000 | 0x0004_0000 | 0x0040_0000;
    let meta = file.metadata().map_err(|_| WhyMemory::NotShown)?;
    if meta.file_attributes() & READ_THROUGH_MEMORY != 0 {
        Err(WhyMemory::StoredSpecially)
    } else if meta.len() < SMALLEST_FROM_DRIVE {
        Err(WhyMemory::TooSmall)
    } else {
        Ok(())
    }
}

/// Whether this very file, opened past the cache on a drive that passed [`probe`], is really read
/// from the drive; else why not. Linux quietly reads some files through its memory even with
/// `O_DIRECT`: those its flags mark inline, journalled, compressed, encrypted or verity-protected.
/// Then the file itself must show a direct read (`statx` alignment, or refusing a read only the
/// cache could do).
#[cfg(target_os = "linux")]
pub(crate) fn this_file_from_drive(file: &std::fs::File) -> Result<(), WhyMemory> {
    use rustix::io::Errno;
    /// ZFS answers a direct read from its own cache when the block is there, so it never counts.
    const ZFS_SUPER_MAGIC: i64 = 0x2fc1_2fc1;
    #[allow(clippy::useless_conversion, clippy::cast_lossless)]
    match rustix::fs::fstatfs(file) {
        Ok(st) if i64::from(st.f_type) == ZFS_SUPER_MAGIC => return Err(WhyMemory::NotShown),
        Ok(_) => {}
        Err(_) => return Err(WhyMemory::NotShown),
    }
    /// `FS_COMPR_FL`, `FS_ENCRYPT_FL`, `FS_JOURNAL_DATA_FL`, `FS_VERITY_FL`, `FS_INLINE_DATA_FL`.
    const READ_THROUGH_MEMORY: u32 =
        0x0000_0004 | 0x0000_0800 | 0x0000_4000 | 0x0010_0000 | 0x1000_0000;
    match rustix::fs::ioctl_getflags(file) {
        Ok(flags) if flags.bits() & READ_THROUGH_MEMORY != 0 => {
            return Err(WhyMemory::StoredSpecially);
        }
        Ok(_) => {}
        // A drive without such flags at all (FAT, exFAT) cannot store a file that way.
        Err(e) if [Errno::NOTTY, Errno::OPNOTSUPP, Errno::INVAL, Errno::NOSYS].contains(&e) => {}
        Err(_) => return Err(WhyMemory::NotShown),
    }
    if direct_io_reported(file) || odd_read_refused(file) {
        Ok(())
    } else {
        Err(WhyMemory::NotShown)
    }
}

/// macOS decides per drive (see [`shown_uncached`]): the gate writes every file with `F_NOCACHE`,
/// so none of its pages are cached for a read to take, and files it writes are never compressed.
#[cfg(not(any(windows, target_os = "linux")))]
pub(crate) fn this_file_from_drive(_file: &std::fs::File) -> Result<(), WhyMemory> {
    Ok(())
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
    let Ok(file) = open_uncached_os(dir, name) else {
        return false;
    };
    #[cfg(target_os = "linux")]
    if direct_io_reported(&file) {
        return true;
    }
    odd_read_refused(&file)
}

/// Whether `file` refuses a read the drive itself could not do (one byte into memory at an odd
/// address), as a handle that bypasses the cache does. On Windows the system refuses it on its
/// length alone, so there it shows only that the handle asks to bypass the cache, not that the
/// file is read that way: [`this_file_from_drive`] decides each file.
#[cfg(any(windows, target_os = "linux"))]
fn odd_read_refused(file: &std::fs::File) -> bool {
    let mut store = vec![0u8; 2 * ALIGN];
    let odd = store.as_ptr().align_offset(ALIGN) + 1;
    matches!(read_once_at_start(file, &mut store[odd..=odd]), Err(e) if e.kind() == io::ErrorKind::InvalidInput)
}

/// One read at the very start of `file`, leaving its position where it was (the same handle is
/// read back from next).
#[cfg(target_os = "linux")]
fn read_once_at_start(file: &std::fs::File, buf: &mut [u8]) -> io::Result<usize> {
    std::os::unix::fs::FileExt::read_at(file, buf, 0)
}

/// One read at the start of `file` (Windows tests only the probe file this way, which is not
/// read again).
#[cfg(windows)]
fn read_once_at_start(mut file: &std::fs::File, buf: &mut [u8]) -> io::Result<usize> {
    file.read(buf)
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
