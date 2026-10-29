//! Waiting sends' copies sealed at rest (Security Design Part G, Part T): each copy is written in
//! chunks, each chunk sealed with ChaCha20-Poly1305 through `snow` (the Noise library Part A
//! names; its own cipher, used as Noise uses it: a 32-byte key and a 64-bit counter nonce). One
//! fresh random key per waiting send, kept only in the system's key store beside the connection
//! keys and deleted with the send, so a deleted send's copy cannot be read again even where the
//! drive keeps the old bytes.
//!
//! The framing is Part T's for chunked data: every chunk's nonce holds the file's number in the
//! send and the chunk's number, and its associated data also says whether it is the last chunk,
//! so a chunk changed, moved to another place or file, cut off or added is refused. The
//! fingerprints of the approval stay over the plain bytes.

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use snow::resolvers::{CryptoResolver, DefaultResolver};
use snow::types::Cipher;
use zeroize::Zeroizing;

/// Plain bytes in each chunk (the last may hold fewer, an empty file one empty chunk).
pub const CHUNK: usize = 64 * 1024;
/// The seal on each chunk.
pub const TAG: usize = 16;
/// Bytes in a send's key.
pub const KEY_LEN: usize = 32;
const LABEL: &[u8] = b"pctwin/v1/waiting-copy";
/// Chunk numbers live in the low 40 bits of the nonce, file numbers in the 24 above.
const CHUNK_BITS: u32 = 40;
/// Most files one sealed send may hold.
pub const MOST_FILES: u32 = 1 << (64 - CHUNK_BITS);

fn damaged(why: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, why)
}

fn cipher(key: &[u8; KEY_LEN]) -> io::Result<Box<dyn Cipher>> {
    let mut c = DefaultResolver
        .resolve_cipher(&snow::params::CipherChoice::ChaChaPoly)
        .ok_or_else(|| io::Error::other("no cipher"))?;
    c.set(key);
    Ok(c)
}

fn nonce(file: u32, chunk: u64) -> io::Result<u64> {
    if file >= MOST_FILES || chunk >= 1 << CHUNK_BITS {
        return Err(damaged("too large to seal"));
    }
    Ok((u64::from(file) << CHUNK_BITS) | chunk)
}

fn associated(file: u32, chunk: u64, last: bool) -> Vec<u8> {
    let mut ad = Vec::with_capacity(LABEL.len() + 13);
    ad.extend_from_slice(LABEL);
    ad.extend_from_slice(&file.to_be_bytes());
    ad.extend_from_slice(&chunk.to_be_bytes());
    ad.push(u8::from(last));
    ad
}

/// How many chunks hold `size` plain bytes.
pub fn chunks(size: u64) -> u64 {
    size.div_ceil(CHUNK as u64).max(1)
}

/// The sealed copy's length on the drive for `size` plain bytes.
pub fn sealed_len(size: u64) -> u64 {
    size.saturating_add(chunks(size).saturating_mul(TAG as u64))
}

/// A fresh send key from the system's random generator.
pub fn new_key() -> io::Result<Zeroizing<[u8; KEY_LEN]>> {
    let mut k = Zeroizing::new([0u8; KEY_LEN]);
    getrandom::fill(k.as_mut()).map_err(|_| io::Error::other("no randomness"))?;
    Ok(k)
}

/// Reads up to `buf.len()` bytes, stopping early only at the end.
fn read_full(r: &mut impl Read, buf: &mut [u8]) -> io::Result<usize> {
    let mut n = 0;
    while n < buf.len() {
        match r.read(&mut buf[n..]) {
            Ok(0) => break,
            Ok(got) => n += got,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(n)
}

/// Seals what `from` holds into `to`, a new file, as file `file` of a send with `key`; returns
/// the plain bytes and their BLAKE3 fingerprint (hex). `renew` is called after every chunk (a
/// long copy keeps its lease).
pub fn seal(
    from: &mut impl Read,
    to: &Path,
    key: &[u8; KEY_LEN],
    file: u32,
    renew: &mut dyn FnMut(),
) -> io::Result<(u64, String)> {
    let c = cipher(key)?;
    let mut dst = File::options().write(true).create_new(true).open(to)?;
    let mut hasher = blake3::Hasher::new();
    let mut cur = vec![0u8; CHUNK];
    let mut next = vec![0u8; CHUNK];
    let mut out = vec![0u8; CHUNK + TAG];
    let mut have = read_full(from, &mut cur)?;
    let mut chunk = 0u64;
    let mut total = 0u64;
    loop {
        // Read ahead: a chunk is the last when nothing follows it.
        let ahead = if have == CHUNK {
            read_full(from, &mut next)?
        } else {
            0
        };
        let last = ahead == 0;
        hasher.update(&cur[..have]);
        let n = c.encrypt(
            nonce(file, chunk)?,
            &associated(file, chunk, last),
            &cur[..have],
            &mut out,
        );
        dst.write_all(&out[..n])?;
        total += have as u64;
        renew();
        if last {
            break;
        }
        std::mem::swap(&mut cur, &mut next);
        have = ahead;
        chunk += 1;
    }
    dst.sync_all()?;
    Ok((total, crate::waiting::hex(hasher.finalize().as_bytes())))
}

/// A sealed copy opened for reading its plain bytes, in any order (a resumed send seeks).
pub struct Unsealed {
    file: File,
    cipher: Box<dyn Cipher>,
    index: u32,
    size: u64,
    pos: u64,
    /// The plain bytes of chunk `held`.
    plain: Vec<u8>,
    held: Option<u64>,
}

impl Unsealed {
    /// The sealed copy at `path`, file `index` of a send with `key`, holding `size` plain bytes:
    /// refused at once when its length on the drive is not what `size` seals to.
    pub fn open(path: &Path, key: &[u8; KEY_LEN], index: u32, size: u64) -> io::Result<Self> {
        let file = File::open(path)?;
        if file.metadata()?.len() != sealed_len(size) {
            return Err(damaged("the sealed copy is not its sealed length"));
        }
        Ok(Self {
            file,
            cipher: cipher(key)?,
            index,
            size,
            pos: 0,
            plain: Vec::new(),
            held: None,
        })
    }

    fn load(&mut self, chunk: u64) -> io::Result<()> {
        if self.held == Some(chunk) {
            return Ok(());
        }
        let last = chunk + 1 == chunks(self.size);
        let start = chunk * CHUNK as u64;
        let plain_len = (self.size - start).min(CHUNK as u64) as usize;
        let mut sealed = vec![0u8; plain_len + TAG];
        self.file
            .seek(SeekFrom::Start(chunk * (CHUNK + TAG) as u64))?;
        self.file.read_exact(&mut sealed)?;
        self.plain.resize(plain_len, 0);
        let n = self
            .cipher
            .decrypt(
                nonce(self.index, chunk)?,
                &associated(self.index, chunk, last),
                &sealed,
                &mut self.plain,
            )
            .map_err(|_| damaged("a sealed chunk was changed"))?;
        if n != plain_len {
            return Err(damaged("a sealed chunk is the wrong length"));
        }
        self.held = Some(chunk);
        Ok(())
    }

    /// The BLAKE3 fingerprint (hex) of every plain byte, read from the start.
    pub fn fingerprint(&mut self) -> io::Result<String> {
        self.seek(SeekFrom::Start(0))?;
        let mut hasher = blake3::Hasher::new();
        let mut buf = vec![0u8; CHUNK];
        loop {
            let got = self.read(&mut buf)?;
            if got == 0 {
                break;
            }
            hasher.update(&buf[..got]);
        }
        self.seek(SeekFrom::Start(0))?;
        Ok(crate::waiting::hex(hasher.finalize().as_bytes()))
    }
}

impl Read for Unsealed {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.pos >= self.size || buf.is_empty() {
            // An empty file is still one sealed chunk: its seal is checked too.
            if self.size == 0 {
                self.load(0)?;
            }
            return Ok(0);
        }
        let chunk = self.pos / CHUNK as u64;
        self.load(chunk)?;
        let at = (self.pos - chunk * CHUNK as u64) as usize;
        let n = (self.plain.len() - at).min(buf.len());
        buf[..n].copy_from_slice(&self.plain[at..at + n]);
        self.pos += n as u64;
        Ok(n)
    }
}

impl Seek for Unsealed {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        let at = match to {
            SeekFrom::Start(n) => Some(n),
            SeekFrom::End(d) => self.size.checked_add_signed(d),
            SeekFrom::Current(d) => self.pos.checked_add_signed(d),
        };
        self.pos = at.ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
        Ok(self.pos)
    }
}

impl pctwin_transfer::Source for Unsealed {
    fn stamp(&mut self) -> io::Result<pctwin_transfer::Stamp> {
        // A sealed copy never changes while it is read: a changed chunk fails its seal instead.
        Ok(pctwin_transfer::Stamp {
            size: self.size,
            modified_ns: Some(0),
        })
    }
}

/// Opens a send's sealed copies, and only those: each by the path its offer reads it from.
pub struct SealedOpener {
    key: Zeroizing<[u8; KEY_LEN]>,
    /// Each copy's path, number in the send and plain size.
    files: Vec<(PathBuf, u32, u64)>,
}

impl SealedOpener {
    pub fn new(key: Zeroizing<[u8; KEY_LEN]>, files: Vec<(PathBuf, u32, u64)>) -> Self {
        Self { key, files }
    }
}

impl pctwin_transfer::Opener for SealedOpener {
    fn open(&self, path: &Path) -> io::Result<Box<dyn pctwin_transfer::Source>> {
        let (_, index, size) = self
            .files
            .iter()
            .find(|(p, _, _)| p == path)
            .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
        Ok(Box::new(Unsealed::open(path, &self.key, *index, *size)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sealed(dir: &Path, plain: &[u8], key: &[u8; 32], index: u32) -> PathBuf {
        let p = dir.join(format!("f{index}"));
        let (n, _) = seal(&mut &plain[..], &p, key, index, &mut || ()).unwrap();
        assert_eq!(n, plain.len() as u64);
        p
    }

    fn all(u: &mut Unsealed) -> io::Result<Vec<u8>> {
        let mut v = Vec::new();
        u.read_to_end(&mut v)?;
        Ok(v)
    }

    #[test]
    fn sealed_bytes_read_back_whole_and_from_anywhere() {
        let d = tempfile::tempdir().unwrap();
        let key = [7u8; 32];
        for size in [0, 1, CHUNK - 1, CHUNK, CHUNK + 1, 3 * CHUNK + 5] {
            let plain: Vec<u8> = (0..size).map(|i| (i * 31 % 251) as u8).collect();
            let p = sealed(d.path(), &plain, &key, size as u32);
            let on_disk = std::fs::read(&p).unwrap();
            assert_eq!(on_disk.len() as u64, sealed_len(size as u64));
            if size > 16 {
                assert!(
                    !on_disk.windows(16).any(|w| w == &plain[..16]),
                    "nothing plain on the drive"
                );
            }
            let mut u = Unsealed::open(&p, &key, size as u32, size as u64).unwrap();
            assert_eq!(all(&mut u).unwrap(), plain);
            assert_eq!(
                u.fingerprint().unwrap(),
                crate::waiting::hex(blake3::hash(&plain).as_bytes())
            );
            if size > 10 {
                u.seek(SeekFrom::Start(size as u64 - 10)).unwrap();
                assert_eq!(all(&mut u).unwrap(), &plain[size - 10..]);
            }
        }
    }

    #[test]
    fn a_changed_moved_cut_or_lengthened_copy_is_refused() {
        let d = tempfile::tempdir().unwrap();
        let key = [7u8; 32];
        let plain = vec![5u8; 2 * CHUNK + 9];
        let size = plain.len() as u64;
        let p = sealed(d.path(), &plain, &key, 3);
        let good = std::fs::read(&p).unwrap();
        // The wrong key, or read as another file of the send.
        assert!(all(&mut Unsealed::open(&p, &[8; 32], 3, size).unwrap()).is_err());
        assert!(all(&mut Unsealed::open(&p, &key, 4, size).unwrap()).is_err());
        // One byte changed.
        let mut bad = good.clone();
        bad[CHUNK + 20] ^= 1;
        std::fs::write(&p, &bad).unwrap();
        assert!(all(&mut Unsealed::open(&p, &key, 3, size).unwrap()).is_err());
        // Two chunks swapped.
        let mut swapped = good.clone();
        let w = CHUNK + TAG;
        let (a, b) = swapped.split_at_mut(w);
        a.swap_with_slice(&mut b[..w]);
        std::fs::write(&p, &swapped).unwrap();
        assert!(all(&mut Unsealed::open(&p, &key, 3, size).unwrap()).is_err());
        // Cut after a whole chunk, and said to be that short: the last chunk is not marked last.
        std::fs::write(&p, &good[..w]).unwrap();
        assert!(all(&mut Unsealed::open(&p, &key, 3, CHUNK as u64).unwrap()).is_err());
        // Cut or lengthened against the approved size: refused at once.
        std::fs::write(&p, &good[..good.len() - 1]).unwrap();
        assert!(Unsealed::open(&p, &key, 3, size).is_err());
        let mut longer = good.clone();
        longer.push(0);
        std::fs::write(&p, &longer).unwrap();
        assert!(Unsealed::open(&p, &key, 3, size).is_err());
        // An empty file's one chunk is checked too.
        let e = sealed(d.path(), b"", &key, 9);
        let mut v = std::fs::read(&e).unwrap();
        v[0] ^= 1;
        std::fs::write(&e, &v).unwrap();
        assert!(all(&mut Unsealed::open(&e, &key, 9, 0).unwrap()).is_err());
    }
}
