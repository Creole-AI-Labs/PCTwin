//! What the new laptop holds while a hostile old laptop sends its list in pages (move protocol 3).
//!
//! The reader counts what each folder and item will take once read, charging every allocation
//! more than any system's allocator takes, and refuses the list past `pctwin_record::MAX_HELD`.
//! This test feeds the heaviest shapes found by the reviews, one shape per process, through the
//! real receive path; `scripts/inventory-memory.py` runs it in release on Windows, macOS and
//! Linux, measures each process's peak (peak commit on Windows, largest resident size on macOS
//! and Linux), and fails if any passes the limit plus the test's own fixed running room.

use std::collections::VecDeque;

use pctwin_transfer::{Channel, ChannelError, PAGED_FROM, TransferError, receive_inventory};

const ROOT: u32 = u32::MAX;
const PIECE: usize = pctwin_transfer::PIECE_MAX;

/// A text name.
fn text(p: &mut Vec<u8>, s: &str) {
    p.push(0);
    p.extend_from_slice(&(s.len() as u32).to_be_bytes());
    p.extend_from_slice(s.as_bytes());
}

/// An exact Windows name of `units` units, the first a lone surrogate, the rest `a`.
fn wide(p: &mut Vec<u8>, units: u32) {
    p.push(2);
    p.extend_from_slice(&units.to_be_bytes());
    for i in 0..units {
        let u: u16 = if i == 0 { 0xD800 } else { 0x61 };
        p.extend_from_slice(&u.to_be_bytes());
    }
}

/// An exact Mac or Linux name of `n` bytes that are not text.
fn unix(p: &mut Vec<u8>, n: u32) {
    p.push(1);
    p.extend_from_slice(&n.to_be_bytes());
    p.extend(std::iter::repeat_n(0xFF, n as usize));
}

/// One item: a unique ID from `seed`, in `folder`, named by `name`.
fn item(p: &mut Vec<u8>, seed: u64, owner: u32, folder: u32, name: &dyn Fn(&mut Vec<u8>)) {
    let mut id = [0u8; 16];
    id[..8].copy_from_slice(&seed.to_be_bytes());
    p.extend_from_slice(&id);
    p.push(0);
    p.extend_from_slice(&owner.to_be_bytes());
    p.extend_from_slice(&0u32.to_be_bytes());
    p.extend_from_slice(&folder.to_be_bytes());
    name(p);
    p.extend_from_slice(&5u64.to_be_bytes());
    p.extend_from_slice(&[0, 0, 0, 0]);
}

/// A shape: how many owners the manifest lists, and how to write page `n` (its folders and
/// items after the header, and how many of each).
struct Shape {
    owners: u32,
    page: fn(u32) -> (u32, u32, u32, Vec<u8>),
}

fn root_items(n: u32, per: u32, name: &dyn Fn(&mut Vec<u8>)) -> (u32, u32, u32, Vec<u8>) {
    let mut p = Vec::new();
    for k in 0..per {
        item(&mut p, u64::from(n) << 32 | u64::from(k), 0, ROOT, name);
    }
    (0, 0, per, p)
}

/// Items at the bottom of a chain of 127 folders called `d`, defined on page 0.
fn deep_items(n: u32, per: u32) -> (u32, u32, u32, Vec<u8>) {
    let mut p = Vec::new();
    let defined = if n == 0 { 127 } else { 0 };
    for i in 0..defined {
        p.extend_from_slice(&(if i == 0 { ROOT } else { i - 1 }).to_be_bytes());
        text(&mut p, "d");
    }
    for k in 0..per {
        item(&mut p, u64::from(n) << 32 | u64::from(k), 0, 126, &|p| {
            text(p, "f")
        });
    }
    (if n == 0 { 0 } else { 127 }, defined, per, p)
}

/// Each item at the bottom of its own chain of 127 folders.
fn folder_chains(n: u32, chains: u32) -> (u32, u32, u32, Vec<u8>) {
    let first = n * chains * 127;
    let mut p = Vec::new();
    for c in 0..chains {
        for i in 0..127u32 {
            let parent = if i == 0 {
                ROOT
            } else {
                first + c * 127 + i - 1
            };
            p.extend_from_slice(&parent.to_be_bytes());
            text(&mut p, &format!("c{n}-{c}"));
        }
    }
    for c in 0..chains {
        let seed = u64::from(n) << 32 | u64::from(c);
        item(&mut p, seed, 0, first + c * 127 + 126, &|p| text(p, "f"));
    }
    (first, chains * 127, chains, p)
}

fn shape(name: &str) -> Shape {
    let page: fn(u32) -> (u32, u32, u32, Vec<u8>) = match name {
        // The longest exact Windows names, at the root.
        "rootwin" => |n| root_items(n, 3_000, &|p| wide(p, 1365)),
        // The longest Mac or Linux names that are not text, at the root.
        "rootunix" => |n| root_items(n, 4_000, &|p| unix(p, 1365)),
        // Windows names of 4,000 units, the first a lone surrogate: blocks of 8 KB, where the
        // Windows heap keeps the most beyond what is asked.
        "wide4000" => |n| root_items(n, 1_000, &|p| wide(p, 4000)),
        // The cheapest items there are.
        "cheapest" => |n| root_items(n, 10_000, &|p| text(p, "f")),
        // Items 128 parts deep in one-byte folders.
        "deep1" => |n| deep_items(n, 1_800),
        // Folder chains, each with one item.
        "folders" => |n| folder_chains(n, 300),
        // 65,536 owners, each item a different one.
        "owners" => |n| {
            let mut p = Vec::new();
            for k in 0..10_000u32 {
                let seed = u64::from(n) << 32 | u64::from(k);
                let owner = ((u64::from(n) * 10_000 + u64::from(k)) % 65_536) as u32;
                item(&mut p, seed, owner, ROOT, &|p| text(p, "f"));
            }
            (0, 0, 10_000, p)
        },
        other => panic!("unknown shape {other}"),
    };
    Shape {
        owners: if name == "owners" { 65_536 } else { 1 },
        page,
    }
}

/// A sender that makes each page only when the last one has been read, so the test itself
/// holds one page at a time.
struct Hostile {
    frames: VecDeque<Vec<u8>>,
    next: i64,
    pages: u32,
    shape: Shape,
}

impl Hostile {
    fn blob(&mut self, index: u32, bytes: Vec<u8>) {
        let mut head = vec![35u8];
        head.extend_from_slice(&index.to_be_bytes());
        head.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
        self.frames.push_back(head);
        for piece in bytes.chunks(PIECE) {
            let mut f = vec![33u8];
            f.extend_from_slice(piece);
            self.frames.push_back(f);
        }
    }

    fn refill(&mut self) {
        if self.next < 0 {
            let mut m = b"PCTWINV2".to_vec();
            m.extend_from_slice(&2u32.to_be_bytes());
            m.extend_from_slice(&[7u8; 16]);
            m.extend_from_slice(&(u64::from(self.pages) * 10_000).to_be_bytes());
            m.extend_from_slice(&self.pages.to_be_bytes());
            m.extend_from_slice(&self.shape.owners.to_be_bytes());
            for i in 0..self.shape.owners {
                if self.shape.owners == 1 {
                    m.push(1); // shared
                } else {
                    let id = format!("{i:010}");
                    m.push(0);
                    m.extend_from_slice(&(id.len() as u32).to_be_bytes());
                    m.extend_from_slice(id.as_bytes());
                }
            }
            m.extend_from_slice(&1u32.to_be_bytes());
            m.extend_from_slice(&[2, 0]); // Documents, on the system drive
            self.blob(u32::MAX, m);
        } else {
            let n = self.next as u32;
            let (first, defined, count, body) = (self.shape.page)(n);
            let mut p = Vec::with_capacity(16 + body.len());
            for v in [n, first, defined, count] {
                p.extend_from_slice(&v.to_be_bytes());
            }
            p.extend_from_slice(&body);
            drop(body);
            self.blob(n, p);
        }
        self.next += 1;
    }
}

impl Channel for Hostile {
    async fn send(&mut self, _: &[u8]) -> Result<(), ChannelError> {
        Ok(())
    }
    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        if self.frames.is_empty() {
            self.refill();
        }
        self.frames.pop_front().ok_or(ChannelError)
    }
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "run in release by scripts/inventory-memory.py, one shape per process"]
async fn the_new_laptop_holds_no_more_than_its_count() {
    println!("max_held {}", pctwin_record::MAX_HELD);
    let name = std::env::var("PCTWIN_MEMORY_SHAPE").unwrap_or_default();
    if name == "baseline" {
        return;
    }
    let mut sender = Hostile {
        frames: VecDeque::new(),
        next: -1,
        pages: 100_000,
        shape: shape(&name),
    };
    let got = receive_inventory(&mut sender, PAGED_FROM).await;
    let pages = sender.next;
    match got {
        Err(TransferError::Inventory(why)) => {
            println!("shape {name}: refused after {pages} pages: {why}");
            assert!(
                why.contains("an inventory that takes more memory than PCTwin allows"),
                "{name}: {why}"
            );
        }
        other => panic!("{name}: expected the list to be refused at the memory limit: {other:?}"),
    }
}
