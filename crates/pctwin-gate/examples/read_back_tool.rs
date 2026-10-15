//! Proof tool for the read-back check: CI and people run it to show, with numbers, that the read
//! back reaches the drive and not just this laptop's cache. Not shipped.
//!
//! Content is deterministic from a seed, so the sender's block hashes can be recomputed in any
//! process. Every run prints one JSON line: `from`, `bad_blocks`, `seconds`, `mb_per_s`.
//!
//!   probe <dir>
//!   write <dir> <path> <bytes> <seed>
//!   write-many <dir> <count> <seed> <manifest>
//!   cached <dir> <stored> <bytes> <seed>
//!   verify <dir> <path> <stored> <bytes> <seed>
//!   cached-many <dir> <manifest> <seed>
//!   verify-many <dir> <manifest> <seed>
//!
//! The manifest has one line per file: `path<TAB>stored<TAB>bytes`.
//! `READ_BACK_BLOCK` (bytes, default 1 MiB) sets the block size.

use std::fmt::Write as _;
use std::io::{Read, Write};
use std::path::Path;
use std::time::Instant;

use pctwin_gate::{CheckedFrom, Destination, IncomingPath};

const PIECE: u64 = 4 * 1024 * 1024;

fn mix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// The bytes of the file `seed` from `offset` (a multiple of 8) for `buf.len()` bytes.
fn fill(seed: u64, offset: u64, buf: &mut [u8]) {
    for (i, chunk) in buf.chunks_mut(8).enumerate() {
        let word = mix(seed ^ mix((offset / 8) + i as u64)).to_le_bytes();
        chunk.copy_from_slice(&word[..chunk.len()]);
    }
}

fn block_size() -> u64 {
    std::env::var("READ_BACK_BLOCK")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&b| b >= 1)
        .unwrap_or(1024 * 1024)
}

/// The sender's hash of every block of the file.
fn sender_hashes(seed: u64, bytes: u64, block: u64) -> Vec<[u8; 32]> {
    let mut out = Vec::new();
    let mut buf = Vec::new();
    let mut at = 0;
    while at < bytes {
        let len = block.min(bytes - at);
        buf.resize(usize::try_from(len).unwrap_or(0), 0);
        // Blocks are multiples of 8 except possibly when READ_BACK_BLOCK is odd; fill by word
        // position so the result stays the same for any block size.
        let start = at - at % 8;
        let mut tmp = vec![0u8; usize::try_from(len + (at - start)).unwrap_or(0)];
        fill(seed, start, &mut tmp);
        buf.copy_from_slice(&tmp[usize::try_from(at - start).unwrap_or(0)..]);
        out.push(*blake3::hash(&buf).as_bytes());
        at += len;
    }
    out
}

struct Run {
    cmd: &'static str,
    from: Option<CheckedFrom>,
    mixed: bool,
    bad: Vec<u64>,
    files: u64,
    bad_files: u64,
    bytes: u64,
    /// Bytes of the files said to be read back from the drive (the CI checks the disk really
    /// read at least that much).
    drive_bytes: u64,
    seconds: f64,
}

impl Run {
    fn print(&self) {
        let mut s = String::new();
        let _ = write!(s, "{{\"cmd\":\"{}\"", self.cmd);
        match self.from {
            Some(f) => {
                let _ = write!(s, ",\"from\":\"{f:?}\"");
            }
            None => s.push_str(",\"from\":null"),
        }
        let _ = write!(s, ",\"mixed_from\":{}", self.mixed);
        let _ = write!(
            s,
            ",\"files\":{},\"bad_files\":{}",
            self.files, self.bad_files
        );
        let list: Vec<String> = self.bad.iter().take(20).map(u64::to_string).collect();
        let _ = write!(
            s,
            ",\"bad_blocks\":{},\"bad_block_list\":[{}]",
            self.bad.len(),
            list.join(",")
        );
        let mb = self.bytes as f64 / 1_000_000.0;
        let _ = write!(
            s,
            ",\"bytes\":{},\"drive_bytes\":{},\"seconds\":{:.3},\"mb_per_s\":{:.1}}}",
            self.bytes,
            self.drive_bytes,
            self.seconds,
            if self.seconds > 0.0 {
                mb / self.seconds
            } else {
                0.0
            }
        );
        println!("{s}");
    }
}

fn die(msg: &str) -> ! {
    eprintln!("read_back_tool: {msg}");
    std::process::exit(2);
}

fn num(s: &str) -> u64 {
    s.parse()
        .unwrap_or_else(|_| die(&format!("not a number: {s}")))
}

fn open(dir: &str) -> Destination {
    Destination::open(Path::new(dir)).unwrap_or_else(|e| die(&format!("open {dir}: {e}")))
}

fn incoming(path: &str) -> IncomingPath {
    IncomingPath::parse(path).unwrap_or_else(|e| die(&format!("path {path}: {e:?}")))
}

/// Writes one file through the gate in 4 MiB pieces and leaves it on disk under its temporary
/// name. Returns the stored path.
fn write_one(dest: &Destination, path: &str, bytes: u64, seed: u64) -> String {
    let mut file = dest
        .create_file(&incoming(path), bytes)
        .unwrap_or_else(|e| die(&format!("create {path}: {e:?}")));
    let mut buf = vec![0u8; usize::try_from(PIECE.min(bytes.max(1))).unwrap_or(0)];
    let mut at = 0;
    while at < bytes {
        let len = PIECE.min(bytes - at);
        let piece = &mut buf[..usize::try_from(len).unwrap_or(0)];
        fill(seed, at, piece);
        file.write_at(at, piece)
            .unwrap_or_else(|e| die(&format!("write {path}: {e:?}")));
        at += len;
    }
    let stored = file.temp_path();
    file.persist();
    stored
}

/// Ordinary cached read of a stored file, compared block by block. Returns the bad blocks.
fn cached_one(dest: &Destination, stored: &str, bytes: u64, seed: u64, block: u64) -> Vec<u64> {
    let hashes = sender_hashes(seed, bytes, block);
    let mut file = dest
        .open_read(stored)
        .unwrap_or_else(|e| die(&format!("open_read {stored}: {e}")));
    let mut buf = vec![0u8; usize::try_from(block).unwrap_or(0)];
    let mut bad = Vec::new();
    for (i, want) in hashes.iter().enumerate() {
        let at = i as u64 * block;
        let len = usize::try_from(block.min(bytes - at)).unwrap_or(0);
        let ok = file.read_exact(&mut buf[..len]).is_ok()
            && blake3::hash(&buf[..len]).as_bytes() == want;
        if !ok {
            bad.push(i as u64);
        }
    }
    bad
}

/// The gate's read-back check of a stored temporary file.
fn verify_one(
    dest: &Destination,
    path: &str,
    stored: &str,
    bytes: u64,
    seed: u64,
    block: u64,
) -> (CheckedFrom, Vec<u64>) {
    let hashes = sender_hashes(seed, bytes, block);
    let mut file = dest
        .reopen_incoming(&incoming(path), stored, bytes)
        .unwrap_or_else(|e| die(&format!("reopen {stored}: {e:?}")));
    file.count_arrived(0, bytes)
        .unwrap_or_else(|e| die(&format!("count_arrived: {e:?}")));
    let v = file
        .verify(block, &hashes)
        .unwrap_or_else(|e| die(&format!("verify {stored}: {e:?}")));
    // Keep the file: later runs read the same one.
    file.persist();
    (v.from, v.bad_blocks)
}

struct Entry {
    path: String,
    stored: String,
    bytes: u64,
}

fn read_manifest(p: &str) -> Vec<Entry> {
    let text = std::fs::read_to_string(p).unwrap_or_else(|e| die(&format!("manifest {p}: {e}")));
    text.lines()
        .filter(|l| !l.is_empty())
        .map(|l| {
            let mut it = l.split('\t');
            let (Some(path), Some(stored), Some(bytes)) = (it.next(), it.next(), it.next()) else {
                die("bad manifest line");
            };
            Entry {
                path: path.to_string(),
                stored: stored.to_string(),
                bytes: num(bytes),
            }
        })
        .collect()
}

/// Seed of the i-th small file.
fn file_seed(seed: u64, i: u64) -> u64 {
    mix(seed ^ i.wrapping_mul(1_000_003))
}

/// Size of the i-th small file: 1 KiB to 256 KiB.
fn file_size(seed: u64, i: u64) -> u64 {
    1024 + mix(seed.wrapping_add(i).wrapping_mul(7919)) % (255 * 1024 + 1)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let a = |i: usize| -> &str {
        args.get(i)
            .map(String::as_str)
            .unwrap_or_else(|| die("missing argument"))
    };
    let block = block_size();
    let cmd = a(0);
    let t = Instant::now();
    let run = match cmd {
        "probe" => {
            let dest = open(a(1));
            let from = dest.read_back_check();
            Run {
                cmd: "probe",
                from: Some(from),
                mixed: false,
                bad: vec![],
                files: 0,
                bad_files: 0,
                bytes: 0,
                drive_bytes: 0,
                seconds: t.elapsed().as_secs_f64(),
            }
        }
        "write" => {
            let (dir, path, bytes, seed) = (a(1), a(2), num(a(3)), num(a(4)));
            let dest = open(dir);
            let stored = write_one(&dest, path, bytes, seed);
            println!("stored={stored}");
            Run {
                cmd: "write",
                from: None,
                mixed: false,
                bad: vec![],
                files: 1,
                bad_files: 0,
                bytes,
                drive_bytes: 0,
                seconds: t.elapsed().as_secs_f64(),
            }
        }
        "write-many" => {
            let (dir, count, seed, manifest) = (a(1), num(a(2)), num(a(3)), a(4));
            let dest = open(dir);
            let mut out = String::new();
            let mut total = 0;
            for i in 0..count {
                let path = format!("many/d{:03}/f{i:05}.bin", i / 100);
                let bytes = file_size(seed, i);
                let stored = write_one(&dest, &path, bytes, file_seed(seed, i));
                let _ = writeln!(out, "{path}\t{stored}\t{bytes}");
                total += bytes;
            }
            std::fs::write(manifest, out).unwrap_or_else(|e| die(&format!("manifest: {e}")));
            Run {
                cmd: "write-many",
                from: None,
                mixed: false,
                bad: vec![],
                files: count,
                bad_files: 0,
                bytes: total,
                drive_bytes: 0,
                seconds: t.elapsed().as_secs_f64(),
            }
        }
        "cached" => {
            let (dir, stored, bytes, seed) = (a(1), a(2), num(a(3)), num(a(4)));
            let dest = open(dir);
            let t = Instant::now();
            let bad = cached_one(&dest, stored, bytes, seed, block);
            Run {
                cmd: "cached",
                from: None,
                mixed: false,
                bad,
                files: 1,
                bad_files: 0,
                bytes,
                drive_bytes: 0,
                seconds: t.elapsed().as_secs_f64(),
            }
        }
        "verify" => {
            let (dir, path, stored, bytes, seed) = (a(1), a(2), a(3), num(a(4)), num(a(5)));
            let dest = open(dir);
            let t = Instant::now();
            let (from, bad) = verify_one(&dest, path, stored, bytes, seed, block);
            Run {
                cmd: "verify",
                from: Some(from),
                mixed: false,
                bad,
                files: 1,
                bad_files: 0,
                bytes,
                drive_bytes: if from == CheckedFrom::Drive { bytes } else { 0 },
                seconds: t.elapsed().as_secs_f64(),
            }
        }
        "cached-many" => {
            let (dir, manifest, seed) = (a(1), a(2), num(a(3)));
            let dest = open(dir);
            let entries = read_manifest(manifest);
            let t = Instant::now();
            let (mut bad, mut bad_files, mut total) = (Vec::new(), 0, 0);
            for (i, e) in entries.iter().enumerate() {
                let b = cached_one(&dest, &e.stored, e.bytes, file_seed(seed, i as u64), block);
                if !b.is_empty() {
                    bad_files += 1;
                    bad.extend(b);
                }
                total += e.bytes;
            }
            Run {
                cmd: "cached-many",
                from: None,
                mixed: false,
                bad,
                files: entries.len() as u64,
                bad_files,
                bytes: total,
                drive_bytes: 0,
                seconds: t.elapsed().as_secs_f64(),
            }
        }
        "verify-many" => {
            let (dir, manifest, seed) = (a(1), a(2), num(a(3)));
            let dest = open(dir);
            let entries = read_manifest(manifest);
            let t = Instant::now();
            let (mut bad, mut bad_files, mut total) = (Vec::new(), 0, 0);
            let (mut first, mut mixed, mut drive_bytes) = (None, false, 0);
            for (i, e) in entries.iter().enumerate() {
                let (from, b) = verify_one(
                    &dest,
                    &e.path,
                    &e.stored,
                    e.bytes,
                    file_seed(seed, i as u64),
                    block,
                );
                match first {
                    None => first = Some(from),
                    Some(f) if f != from => mixed = true,
                    _ => {}
                }
                if from == CheckedFrom::Drive {
                    drive_bytes += e.bytes;
                }
                if !b.is_empty() {
                    bad_files += 1;
                    bad.extend(b);
                }
                total += e.bytes;
            }
            Run {
                cmd: "verify-many",
                from: first,
                mixed,
                bad,
                files: entries.len() as u64,
                bad_files,
                bytes: total,
                drive_bytes,
                seconds: t.elapsed().as_secs_f64(),
            }
        }
        other => die(&format!("unknown subcommand {other}")),
    };
    run.print();
    let _ = std::io::stdout().flush();
}
