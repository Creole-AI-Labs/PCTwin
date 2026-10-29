//! Waiting sends sealed at rest (Security Design Part G, Part T): every copy, and the list of what
//! the send holds, is encrypted with RustCrypto's `chacha20poly1305` through its STREAM
//! construction (`aead::stream`, the published online-encryption scheme of Hoang, Reyhanitabar,
//! Rogaway and Vizár), the one Part G names: no chunk framing of PCTwin's own. STREAM seals each
//! chunk with a counter and a last-chunk mark in its nonce, so a chunk changed, reordered, cut off
//! or added is refused. One fresh random key per waiting send, kept only in the system's key store
//! and deleted with the send; each sealed thing in the send is its own stream under that key
//! (its own nonce prefix), so a copy moved to another file's place is refused too.
//!
//! Reading seeks by chunk: the stream's own position-addressed decryption (STREAM's counter set
//! to the chunk's number) opens any chunk, which a resumed send needs.

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use chacha20poly1305::aead::stream::{EncryptorBE32, NewStream, StreamBE32, StreamPrimitive};
use chacha20poly1305::{ChaCha20Poly1305, KeyInit};
use zeroize::Zeroizing;

/// Plain bytes in each chunk (the last may hold fewer, an empty stream one empty chunk).
pub const CHUNK: usize = 64 * 1024;
/// The seal on each chunk.
pub const TAG: usize = 16;
/// Bytes in a send's key.
pub const KEY_LEN: usize = 32;
/// The stream that holds the send's own list of what it holds; each copied file has its own
/// ([`file_stream`]), never this one.
pub const LIST_STREAM: u32 = 0;

/// The stream of a send's file number `n` (from 0): never [`LIST_STREAM`], never the same for two
/// files. `None` past the most files one send can seal.
pub fn file_stream(n: usize) -> Option<u32> {
    u32::try_from(n)
        .ok()
        .and_then(|n| n.checked_add(1))
        .filter(|s| *s != LIST_STREAM)
}

fn damaged(why: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, why)
}

/// The STREAM nonce prefix (7 bytes) of stream `id` in a send: unique under the send's key.
fn prefix(id: u32) -> [u8; 7] {
    let mut p = [0u8; 7];
    p[3..].copy_from_slice(&id.to_be_bytes());
    p
}

fn stream(key: &[u8; KEY_LEN], id: u32) -> StreamBE32<ChaCha20Poly1305> {
    let aead = ChaCha20Poly1305::new(key.into());
    StreamBE32::from_aead(aead, (&prefix(id)).into())
}

/// How many chunks hold `size` plain bytes.
pub fn chunks(size: u64) -> u64 {
    size.div_ceil(CHUNK as u64).max(1)
}

/// The sealed length on the drive for `size` plain bytes.
pub fn sealed_len(size: u64) -> u64 {
    size.saturating_add(chunks(size).saturating_mul(TAG as u64))
}

/// The plain size a sealed length stands for, when it is one.
fn plain_len(sealed: u64) -> Option<u64> {
    let whole = CHUNK as u64 + TAG as u64;
    let n = sealed.div_ceil(whole).max(1);
    let size = sealed.checked_sub(n * TAG as u64)?;
    (sealed_len(size) == sealed).then_some(size)
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

/// Seals what `from` holds into `to`, a new file, as stream `id` of a send with `key`; returns the
/// plain bytes and their BLAKE3 fingerprint (hex). `renew` is called after every chunk (a long
/// copy keeps its lease). Each stream id is sealed once only.
pub fn seal(
    from: &mut impl Read,
    to: &Path,
    key: &[u8; KEY_LEN],
    id: u32,
    renew: &mut dyn FnMut(),
) -> io::Result<(u64, String)> {
    let mut dst = File::options().write(true).create_new(true).open(to)?;
    let mut enc = Some(EncryptorBE32::from_aead(
        ChaCha20Poly1305::new(key.into()),
        (&prefix(id)).into(),
    ));
    let mut hasher = blake3::Hasher::new();
    let mut cur = vec![0u8; CHUNK];
    let mut next = vec![0u8; CHUNK];
    let mut have = read_full(from, &mut cur)?;
    let mut total = 0u64;
    loop {
        // Read ahead: a chunk is the last when nothing follows it.
        let ahead = if have == CHUNK {
            read_full(from, &mut next)?
        } else {
            0
        };
        hasher.update(&cur[..have]);
        let mut sealed = Vec::with_capacity(have + TAG);
        sealed.extend_from_slice(&cur[..have]);
        let sealing = if ahead == 0 {
            enc.take()
                .ok_or_else(|| io::Error::other("sealed already"))?
                .encrypt_last_in_place(b"", &mut sealed)
        } else {
            enc.as_mut()
                .ok_or_else(|| io::Error::other("sealed already"))?
                .encrypt_next_in_place(b"", &mut sealed)
        };
        sealing.map_err(|_| io::Error::other("too long to seal"))?;
        dst.write_all(&sealed)?;
        total += have as u64;
        renew();
        if ahead == 0 {
            break;
        }
        std::mem::swap(&mut cur, &mut next);
        have = ahead;
    }
    dst.sync_all()?;
    Ok((total, crate::waiting::hex(hasher.finalize().as_bytes())))
}

/// Seals `bytes` into `to` (a new file) as stream `id`.
pub fn seal_bytes(bytes: &[u8], to: &Path, key: &[u8; KEY_LEN], id: u32) -> io::Result<()> {
    seal(&mut &bytes[..], to, key, id, &mut || ()).map(|_| ())
}

/// Every plain byte of the sealed stream `id` at `path`, at most `most` of them.
pub fn unseal_bytes(path: &Path, key: &[u8; KEY_LEN], id: u32, most: u64) -> io::Result<Vec<u8>> {
    let len = std::fs::metadata(path)?.len();
    let size = plain_len(len).ok_or_else(|| damaged("not a sealed length"))?;
    if size > most {
        return Err(damaged("too large"));
    }
    let mut u = Unsealed::open(path, key, id, size)?;
    let mut out = Vec::new();
    u.read_to_end(&mut out)?;
    Ok(out)
}

/// A sealed copy opened for reading its plain bytes, in any order (a resumed send seeks).
pub struct Unsealed {
    file: File,
    stream: StreamBE32<ChaCha20Poly1305>,
    size: u64,
    pos: u64,
    /// The plain bytes of chunk `held`.
    plain: Vec<u8>,
    held: Option<u64>,
}

impl Unsealed {
    /// The sealed stream `id` at `path`, of a send with `key`, holding `size` plain bytes: refused
    /// at once when its length on the drive is not what `size` seals to.
    pub fn open(path: &Path, key: &[u8; KEY_LEN], id: u32, size: u64) -> io::Result<Self> {
        let file = File::open(path)?;
        if file.metadata()?.len() != sealed_len(size) {
            return Err(damaged("the sealed copy is not its sealed length"));
        }
        if chunks(size) > u64::from(u32::MAX) {
            return Err(damaged("too large"));
        }
        Ok(Self {
            file,
            stream: stream(key, id),
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
        let mut buf = vec![0u8; plain_len + TAG];
        self.file
            .seek(SeekFrom::Start(chunk * (CHUNK + TAG) as u64))?;
        self.file.read_exact(&mut buf)?;
        let position = u32::try_from(chunk).map_err(|_| damaged("too large"))?;
        self.stream
            .decrypt_in_place(position, last, b"", &mut buf)
            .map_err(|_| damaged("a sealed chunk was changed"))?;
        self.plain = buf;
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
            // An empty stream is still one sealed chunk: its seal is checked too.
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
    /// Each copy's path, stream id and plain size.
    files: Vec<(PathBuf, u32, u64)>,
}

impl SealedOpener {
    pub fn new(key: Zeroizing<[u8; KEY_LEN]>, files: Vec<(PathBuf, u32, u64)>) -> Self {
        Self { key, files }
    }
}

impl pctwin_transfer::Opener for SealedOpener {
    fn open(&self, path: &Path) -> io::Result<Box<dyn pctwin_transfer::Source>> {
        let (_, id, size) = self
            .files
            .iter()
            .find(|(p, _, _)| p == path)
            .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
        Ok(Box::new(Unsealed::open(path, &self.key, *id, *size)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sealed(dir: &Path, plain: &[u8], key: &[u8; 32], id: u32) -> PathBuf {
        let p = dir.join(format!("f{id}"));
        let (n, _) = seal(&mut &plain[..], &p, key, id, &mut || ()).unwrap();
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
            assert_eq!(plain_len(on_disk.len() as u64), Some(size as u64));
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
            assert_eq!(
                unseal_bytes(&p, &key, size as u32, u64::MAX).unwrap(),
                plain
            );
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
        // The wrong key, or read as another file of the send (swapped between files).
        assert!(all(&mut Unsealed::open(&p, &[8; 32], 3, size).unwrap()).is_err());
        assert!(all(&mut Unsealed::open(&p, &key, 4, size).unwrap()).is_err());
        // One byte changed.
        let mut bad = good.clone();
        bad[CHUNK + 20] ^= 1;
        std::fs::write(&p, &bad).unwrap();
        assert!(all(&mut Unsealed::open(&p, &key, 3, size).unwrap()).is_err());
        // Two chunks reordered.
        let mut swapped = good.clone();
        let w = CHUNK + TAG;
        let (a, b) = swapped.split_at_mut(w);
        a.swap_with_slice(&mut b[..w]);
        std::fs::write(&p, &swapped).unwrap();
        assert!(all(&mut Unsealed::open(&p, &key, 3, size).unwrap()).is_err());
        // Cut after a whole chunk, and said to be that short: the last chunk is not marked last.
        std::fs::write(&p, &good[..w]).unwrap();
        assert!(all(&mut Unsealed::open(&p, &key, 3, CHUNK as u64).unwrap()).is_err());
        assert!(unseal_bytes(&p, &key, 3, u64::MAX).is_err());
        // Cut or lengthened against the approved size: refused at once.
        std::fs::write(&p, &good[..good.len() - 1]).unwrap();
        assert!(Unsealed::open(&p, &key, 3, size).is_err());
        let mut longer = good.clone();
        longer.push(0);
        std::fs::write(&p, &longer).unwrap();
        assert!(Unsealed::open(&p, &key, 3, size).is_err());
        // An empty stream's one chunk is checked too.
        let e = sealed(d.path(), b"", &key, 9);
        let mut v = std::fs::read(&e).unwrap();
        v[0] ^= 1;
        std::fs::write(&e, &v).unwrap();
        assert!(all(&mut Unsealed::open(&e, &key, 9, 0).unwrap()).is_err());
    }

    /// A file's stream is never the list's: a copy put in the list's place does not open as the
    /// list, nor the list as a copy.
    #[test]
    fn a_file_never_opens_as_the_list() {
        assert_eq!(file_stream(0), Some(1));
        assert_ne!(file_stream(0), Some(LIST_STREAM));
        assert_ne!(file_stream(1), file_stream(0));
        assert_eq!(file_stream(u32::MAX as usize), None);
        let d = tempfile::tempdir().unwrap();
        let key = [7u8; 32];
        let first = d.path().join("0.sealed");
        seal_bytes(b"[]", &first, &key, file_stream(0).unwrap()).unwrap();
        assert!(unseal_bytes(&first, &key, LIST_STREAM, 100).is_err());
        let list = d.path().join("list.sealed");
        seal_bytes(b"[]", &list, &key, LIST_STREAM).unwrap();
        assert!(unseal_bytes(&list, &key, file_stream(0).unwrap(), 100).is_err());
    }

    /// The same bytes in two streams of one send seal differently, so neither stands in for the
    /// other.
    #[test]
    fn each_stream_of_a_send_seals_on_its_own() {
        let d = tempfile::tempdir().unwrap();
        let key = [7u8; 32];
        let a = std::fs::read(sealed(d.path(), b"same bytes", &key, 1)).unwrap();
        let b = std::fs::read(sealed(d.path(), b"same bytes", &key, 2)).unwrap();
        assert_ne!(a, b);
        assert!(unseal_bytes(&d.path().join("f1"), &key, 2, 100).is_err());
        assert!(
            unseal_bytes(&d.path().join("f1"), &key, 1, 5).is_err(),
            "too large"
        );
    }
}
