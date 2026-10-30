//! A fuzz of every reader the parser process runs, on the stable compiler (cargo-fuzz needs a
//! nightly one, which CI has and this check does not need): seeds (made here, and on Windows real
//! signed programs and installer packages), mutated at random for a bounded time, each read
//! through the parser's own code path ([`answer_in_memory`]). A panic is a crash the parser
//! process would have had: its input is kept under `fuzz/artifacts/stable/` and the test fails.
//!
//! Time: `PCTWIN_FUZZ_SECS` (default 5 seconds per reader). Seed of the random choices:
//! `PCTWIN_FUZZ_SEED` (default: from the clock, printed).
#![expect(
    clippy::disallowed_methods,
    reason = "the test reads seed files and keeps crashing inputs, as the app side would"
)]

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use pctwin_parse::testing::answer_in_memory;
use pctwin_parse::wire::Request;

/// xorshift64*: enough to choose mutations; nothing here is secret.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

fn tiny_pe() -> Vec<u8> {
    let mut b = vec![0u8; 0x400];
    b[0] = b'M';
    b[1] = b'Z';
    b[0x3C..0x40].copy_from_slice(&0x80u32.to_le_bytes());
    b[0x80..0x84].copy_from_slice(b"PE\0\0");
    let coff = 0x84;
    b[coff..coff + 2].copy_from_slice(&0x8664u16.to_le_bytes());
    b[coff + 2..coff + 4].copy_from_slice(&1u16.to_le_bytes());
    b[coff + 16..coff + 18].copy_from_slice(&240u16.to_le_bytes());
    let opt = coff + 20;
    b[opt..opt + 2].copy_from_slice(&0x20Bu16.to_le_bytes());
    b[opt + 60..opt + 64].copy_from_slice(&0x200u32.to_le_bytes());
    b[opt + 108..opt + 112].copy_from_slice(&16u32.to_le_bytes());
    let sec = opt + 240;
    b[sec + 16..sec + 20].copy_from_slice(&0x200u32.to_le_bytes());
    b[sec + 20..sec + 24].copy_from_slice(&0x200u32.to_le_bytes());
    b
}

fn made_seeds() -> Vec<Vec<u8>> {
    let mut sqlite = vec![0u8; 4096];
    sqlite[..16].copy_from_slice(b"SQLite format 3\0");
    sqlite[16..18].copy_from_slice(&4096u16.to_be_bytes());
    sqlite[18] = 1;
    sqlite[19] = 1;
    sqlite[21] = 64;
    sqlite[22] = 32;
    sqlite[23] = 32;
    let mut zip = b"PK\x03\x04".to_vec();
    zip.resize(40, 0);
    zip.extend_from_slice(b"PK\x01\x02");
    zip.resize(80, 0);
    let mut end = b"PK\x05\x06\0\0\0\0\x01\0\x01\0".to_vec();
    end.extend_from_slice(&40u32.to_le_bytes());
    end.extend_from_slice(&40u32.to_le_bytes());
    end.extend_from_slice(&[0, 0]);
    zip.extend_from_slice(&end);
    // A TrueType font with one table, `name`, holding one family name.
    let mut ttf = vec![0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0];
    ttf.extend_from_slice(b"name\0\0\0\0");
    ttf.extend_from_slice(&28u32.to_be_bytes());
    ttf.extend_from_slice(&30u32.to_be_bytes());
    ttf.extend_from_slice(&[0, 0, 0, 1, 0, 18, 0, 3, 0, 1, 0, 0, 0, 1, 0, 4, 0, 0]);
    ttf.extend_from_slice(&[0, b'A', 0, b'b']);
    let mut seeds = vec![
        tiny_pe(),
        sqlite,
        zip,
        ttf,
        b"ttcf\0\x01\0\0\0\0\0\x01\0\0\0\x10".to_vec(),
        b"[Flatpak Ref]\nName=org.example.App\nUrl=https://dl.example.org/repo/\nGPGKey=AAAA\n"
            .to_vec(),
    ];
    // An empty installer package (a compound file with the installer's class ID).
    seeds.push({
        let mut b = vec![0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
        b.resize(1536, 0);
        b
    });
    // A launcher's records (Valve's text key-values, JSON) and a save list in the Ludusavi
    // manifest format after the games asked about (2.5 games).
    seeds.push(
        b"\"libraryfolders\" { \"0\" { \"path\" \"C:\\Steam\" \"apps\" { \"7\" \"1\" } } }"
            .to_vec(),
    );
    seeds.push(br#"{"DisplayName":"A","InstallLocation":"D:\A","List":[1,true,null]}"#.to_vec());
    let mut list = WANTED.to_vec();
    list.extend_from_slice(
        b"---
\"A: B\":
  files:
    <winAppData>/A/*.sav:
      tags:
        - save
      when:
        - os: windows
  steam:
    id: 1
'C''d':
  cloud:
    steam: true
",
    );
    seeds.push(list);
    seeds
}

/// The games a save list is read for, at the start of every save list seed.
const WANTED: &[u8] = br#"{"names":["a"],"steam":[1],"gog":[]}"#;

/// Real signed programs and packages from this laptop (Windows only), up to `most`, each at most
/// 4 MiB: the deep parts of the signature reader (CMS, X.509, the compound file).
fn real_seeds(most: usize) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    if !cfg!(windows) {
        return out;
    }
    let windir = std::env::var_os("WINDIR").unwrap_or_else(|| "C:\\Windows".into());
    for dir in ["System32", "Installer"] {
        let Ok(list) = std::fs::read_dir(PathBuf::from(&windir).join(dir)) else {
            continue;
        };
        for e in list.flatten() {
            if out.len() >= most {
                return out;
            }
            let p = e.path();
            let ok = p.extension().is_some_and(|x| {
                x.eq_ignore_ascii_case("exe")
                    || x.eq_ignore_ascii_case("msi")
                    || x.eq_ignore_ascii_case("ttf")
            });
            // Only those carrying a signature (a DER sequence): the rest are covered by the made
            // seeds.
            if ok
                && std::fs::metadata(&p).is_ok_and(|m| m.len() < 4 * 1024 * 1024)
                && let Ok(b) = std::fs::read(&p)
                && b.windows(2).any(|w| w == [0x30, 0x82])
            {
                out.push(b);
            }
        }
    }
    out
}

fn mutate(rng: &mut Rng, seed: &[u8]) -> Vec<u8> {
    let mut b = seed.to_vec();
    let rounds = 1 + rng.below(8);
    for _ in 0..rounds {
        match rng.below(6) {
            0 if !b.is_empty() => {
                let i = rng.below(b.len());
                b[i] = rng.next() as u8;
            }
            1 if !b.is_empty() => {
                // A small or large number where a length or offset is likely.
                let i = rng.below(b.len().saturating_sub(4).max(1));
                let v: u32 = match rng.below(4) {
                    0 => 0,
                    1 => u32::MAX,
                    2 => rng.next() as u32,
                    _ => b.len() as u32 + rng.below(16) as u32,
                };
                for (k, x) in v.to_le_bytes().iter().enumerate() {
                    if let Some(slot) = b.get_mut(i + k) {
                        *slot = *x;
                    }
                }
            }
            2 => {
                let cut = rng.below(b.len() + 1);
                b.truncate(cut);
            }
            3 => {
                let n = rng.below(512);
                b.extend((0..n).map(|_| rng.next() as u8));
            }
            4 if b.len() > 8 => {
                // Copy a run onto another place (repeated structures).
                let from = rng.below(b.len());
                let to = rng.below(b.len());
                let n = rng.below(64).min(b.len() - from).min(b.len() - to);
                let run = b[from..from + n].to_vec();
                b[to..to + n].copy_from_slice(&run);
            }
            _ => {}
        }
    }
    b
}

fn keep(input: &[u8], request: &Request) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fuzz/artifacts/stable")
        .canonicalize()
        .unwrap_or_else(|_| {
            let d = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/artifacts/stable");
            std::fs::create_dir_all(&d).unwrap();
            d
        });
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in input {
        h = (h ^ u64::from(*b)).wrapping_mul(0x100_0000_01b3);
    }
    let kind = format!("{request:?}");
    let kind = kind.split_whitespace().next().unwrap_or("x");
    let p = dir.join(format!("{kind}-{h:016x}"));
    std::fs::write(&p, input).unwrap();
    p
}

#[test]
fn every_reader_survives_mutated_files() {
    let secs: u64 = std::env::var("PCTWIN_FUZZ_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5);
    let seed: u64 = std::env::var("PCTWIN_FUZZ_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(1, |d| d.as_nanos() as u64)
                | 1
        });
    eprintln!("fuzz seed {seed}, {secs} s per reader");
    let mut rng = Rng(seed);
    let mut seeds = made_seeds();
    seeds.extend(real_seeds(40));
    let kinds: [fn(u64) -> Request; 10] = [
        |size| Request::Installer { size },
        |size| Request::OldCopy { size },
        |size| Request::Font { size },
        |size| Request::Archive { size },
        |size| Request::Database { size },
        |size| Request::Json { size },
        |size| Request::DatabaseFacts {
            size,
            log: 0,
            millis: 2000,
        },
        |size| Request::GameRecord {
            size,
            format: pctwin_special::gamefmt::RecordFormat::Vdf,
        },
        |size| Request::GameRecord {
            size,
            format: pctwin_special::gamefmt::RecordFormat::Json,
        },
        |size| {
            let wanted = size.min(WANTED.len() as u64);
            Request::SaveManifest {
                wanted,
                size: size - wanted,
            }
        },
    ];
    let mut runs = 0u64;
    let mut found = Vec::new();
    for make in kinds {
        let until = Instant::now() + Duration::from_secs(secs);
        while Instant::now() < until {
            let base = &seeds[rng.below(seeds.len())];
            let input = mutate(&mut rng, base);
            let request = make(input.len() as u64);
            runs += 1;
            let got = catch_unwind(AssertUnwindSafe(|| {
                answer_in_memory(pctwin_parse_seal::in_child(), request, &input)
            }));
            match got {
                Ok(answer) => assert!(answer.answers(&request)),
                Err(_) => found.push(keep(&input, &request)),
            }
        }
    }
    eprintln!("{runs} inputs read");
    assert!(found.is_empty(), "crashes kept: {found:?}");
}
