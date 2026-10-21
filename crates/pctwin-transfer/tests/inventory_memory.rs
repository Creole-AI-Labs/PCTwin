//! What the new laptop holds while a hostile old laptop sends its list in pages (move protocol 3),
//! and afterwards, while the list it accepted is approved, planned and saved.
//!
//! The reader counts what each folder and item will take once read, charging every allocation
//! more than any system's allocator takes, and refuses the list past `pctwin_record::MAX_HELD`.
//! This test feeds the heaviest shapes found by the reviews, one shape per process, through the
//! real receive path. Some are refused at the limit; the `accept-…` shapes send a list just within
//! it and then approve it, plan it (where every file lands) or save it, so the whole path is
//! measured, not only the reader. `scripts/inventory-memory.py` runs it in release on Windows,
//! macOS and Linux, measures each process's peak (peak commit on Windows, the largest memory
//! footprint on macOS, the largest resident size on Linux), and fails if any passes the limit
//! plus the test's own fixed running room.

use std::collections::VecDeque;

use pctwin_transfer::{
    Allowance, Channel, ChannelError, LandingPlace, PAGED_FROM, TransferError, receive_inventory,
    shown_path,
};

const ROOT: u32 = u32::MAX;
const PIECE: usize = pctwin_transfer::PIECE_MAX;

/// A text name.
fn text(p: &mut Vec<u8>, s: &str) {
    p.push(0);
    p.extend_from_slice(&(s.len() as u32).to_be_bytes());
    p.extend_from_slice(s.as_bytes());
}

/// An exact Windows name of `units` units, the first `lone` of them lone surrogates (each shown
/// as three bytes), the rest `a`.
fn wide(p: &mut Vec<u8>, units: u32, lone: u32) {
    p.push(2);
    p.extend_from_slice(&units.to_be_bytes());
    for i in 0..units {
        let u: u16 = if i < lone { 0xD800 } else { 0x61 };
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
    /// Items in every page.
    per: u32,
    page: fn(u32, &mut Vec<u8>) -> (u32, u32, u32),
}

fn root_items(n: u32, per: u32, name: &dyn Fn(&mut Vec<u8>), p: &mut Vec<u8>) -> (u32, u32, u32) {
    for k in 0..per {
        item(p, u64::from(n) << 32 | u64::from(k), 0, ROOT, name);
    }
    (0, 0, per)
}

/// Items at the bottom of a chain of 127 folders called `d`, defined on page 0.
fn deep_items(n: u32, per: u32, p: &mut Vec<u8>) -> (u32, u32, u32) {
    let defined = if n == 0 { 127 } else { 0 };
    for i in 0..defined {
        p.extend_from_slice(&(if i == 0 { ROOT } else { i - 1 }).to_be_bytes());
        text(p, "d");
    }
    for k in 0..per {
        item(p, u64::from(n) << 32 | u64::from(k), 0, 126, &|p| {
            text(p, "f")
        });
    }
    (if n == 0 { 0 } else { 127 }, defined, per)
}

/// Each item at the bottom of its own chain of 127 folders.
fn folder_chains(n: u32, chains: u32, p: &mut Vec<u8>) -> (u32, u32, u32) {
    let first = n * chains * 127;
    for c in 0..chains {
        for i in 0..127u32 {
            let parent = if i == 0 {
                ROOT
            } else {
                first + c * 127 + i - 1
            };
            p.extend_from_slice(&parent.to_be_bytes());
            text(p, &format!("c{n}-{c}"));
        }
    }
    for c in 0..chains {
        let seed = u64::from(n) << 32 | u64::from(c);
        item(p, seed, 0, first + c * 127 + 126, &|p| text(p, "f"));
    }
    (first, chains * 127, chains)
}

/// Items in three folders at the root, defined on page 0, each named by 1,025 exact Windows units
/// (one lone surrogate): every item carries a copy of its folder's name, in blocks of about 1.5 and
/// 2.5 KB, where macOS rounds up to multiples of 512.
fn shared_folders(n: u32, per: u32, p: &mut Vec<u8>) -> (u32, u32, u32) {
    let defined = if n == 0 { 3 } else { 0 };
    for f in 0..defined {
        p.extend_from_slice(&ROOT.to_be_bytes());
        p.push(2);
        p.extend_from_slice(&1025u32.to_be_bytes());
        p.extend_from_slice(&0xD800u16.to_be_bytes());
        for _ in 0..1024 {
            p.extend_from_slice(&(0x61 + f as u16).to_be_bytes());
        }
    }
    for k in 0..per {
        item(p, u64::from(n) << 32 | u64::from(k), 0, k % 3, &|p| {
            text(p, "f")
        });
    }
    (if n == 0 { 0 } else { 3 }, defined, per)
}

fn shape(name: &str) -> Shape {
    let per = match name {
        "rootwin" => 3_000,
        "rootunix" => 4_000,
        "wide4000" => 1_000,
        "deep1" => 1_800,
        "folders" => 300,
        "sharedwin" => 6_000,
        "roottext" => 2_000,
        _ => 10_000,
    };
    let page: fn(u32, &mut Vec<u8>) -> (u32, u32, u32) = match name {
        // The longest exact Windows names, at the root.
        "rootwin" => |n, page| root_items(n, 3_000, &|p| wide(p, 1365, 1365), page),
        // The longest Mac or Linux names that are not text, at the root.
        "rootunix" => |n, page| root_items(n, 4_000, &|p| unix(p, 1365), page),
        // Windows names of 4,000 units, the first a lone surrogate: blocks of 8 KB, where the
        // Windows heap keeps the most beyond what is asked.
        "wide4000" => |n, page| root_items(n, 1_000, &|p| wide(p, 4000, 1), page),
        // The cheapest items there are.
        "cheapest" => |n, page| root_items(n, 10_000, &|p| text(p, "f"), page),
        // Items 128 parts deep in one-byte folders.
        "deep1" => |n, page| deep_items(n, 1_800, page),
        // Folder chains, each with one item.
        "folders" => |n, page| folder_chains(n, 300, page),
        // The longest plain names, at the root: what a name holds is all text, which a plan
        // repeats in where each file lands.
        "roottext" => |n, page| {
            let name = "a".repeat(4095);
            root_items(n, 2_000, &|p| text(p, &name), page)
        },
        // Many tiny items in three folders with long exact Windows names.
        "sharedwin" => |n, page| shared_folders(n, 6_000, page),
        // 65,536 owners, each item a different one.
        "owners" => |n, p| {
            for k in 0..10_000u32 {
                let seed = u64::from(n) << 32 | u64::from(k);
                let owner = ((u64::from(n) * 10_000 + u64::from(k)) % 65_536) as u32;
                item(p, seed, owner, ROOT, &|p| text(p, "f"));
            }
            (0, 0, 10_000)
        },
        other => panic!("unknown shape {other}"),
    };
    Shape {
        owners: if name == "owners" { 65_536 } else { 1 },
        per,
        page,
    }
}

/// A sender that makes each page only when the last one has been read, into one buffer it keeps,
/// so the test itself holds one page at a time and makes no page buffer to let go each time.
struct Hostile {
    frames: VecDeque<Vec<u8>>,
    page: Vec<u8>,
    next: i64,
    pages: u32,
    shape: Shape,
}

impl Hostile {
    fn blob(&mut self, index: u32) {
        let mut head = vec![35u8];
        head.extend_from_slice(&index.to_be_bytes());
        head.extend_from_slice(&(self.page.len() as u32).to_be_bytes());
        self.frames.push_back(head);
        for piece in self.page.chunks(PIECE) {
            let mut f = Vec::with_capacity(1 + piece.len());
            f.push(33u8);
            f.extend_from_slice(piece);
            self.frames.push_back(f);
        }
    }

    fn refill(&mut self) {
        if self.next < 0 {
            let mut m = b"PCTWINV2".to_vec();
            m.extend_from_slice(&2u32.to_be_bytes());
            m.extend_from_slice(&[7u8; 16]);
            m.extend_from_slice(&(u64::from(self.pages) * u64::from(self.shape.per)).to_be_bytes());
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
            self.page = m;
            self.blob(u32::MAX);
        } else {
            let n = self.next as u32;
            self.page.clear();
            for v in [n, 0, 0, 0] {
                self.page.extend_from_slice(&v.to_be_bytes());
            }
            let (first, defined, count) = (self.shape.page)(n, &mut self.page);
            for (at, v) in [(4, first), (8, defined), (12, count)] {
                self.page[at..at + 4].copy_from_slice(&v.to_be_bytes());
            }
            self.blob(n);
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

/// What is done with a list that was accepted.
#[derive(Clone, Copy, PartialEq)]
enum After {
    /// Approve it, as the person does once everyone has a place (the fingerprint included).
    Approve,
    /// Approve it, then work out where every file lands (the plan the new laptop holds).
    Plan,
    /// Approve it, then save it to a file.
    Persist,
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "run in release by scripts/inventory-memory.py, one shape per process"]
async fn the_new_laptop_holds_no_more_than_its_count() {
    println!("max_held {}", pctwin_record::MAX_HELD);
    let name = std::env::var("PCTWIN_MEMORY_SHAPE").unwrap_or_default();
    if name != "baseline" {
        run_shape(&name).await;
    }
    // On macOS the script reads the process's largest footprint while it still runs: the work
    // is done, so it says so and waits until the script has read it.
    if std::env::var_os("PCTWIN_MEMORY_HOLD").is_some() {
        println!("memory: done");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        let mut rest = Vec::new();
        std::io::Read::read_to_end(&mut std::io::stdin(), &mut rest).unwrap();
    }
}

/// Sends the list of shape `name`, then refuses or uses it as the shape says.
async fn run_shape(name: &str) {
    // A list just within the limit: 47 pages of 3,000 items at the root, each named by 1,365
    // lone surrogates (about 1.32 GB as the reader counts it), or 110 pages of 2,000 with plain
    // names of 4,095 bytes, accepted, then used.
    let after = match name {
        "accept-approve" => Some(After::Approve),
        "accept-plan" => Some(After::Plan),
        "accept-persist" => Some(After::Persist),
        "accept-plan-text" => Some(After::Plan),
        _ => None,
    };
    let (base, pages) = match (after, name) {
        (Some(_), "accept-plan-text") => ("roottext", 110),
        (Some(_), _) => ("rootwin", 47),
        (None, _) => (name, 100_000),
    };
    let mut sender = Hostile {
        frames: VecDeque::new(),
        page: Vec::with_capacity(pctwin_record::MAX_PAGE_BYTES),
        next: -1,
        pages,
        shape: shape(base),
    };
    let got = receive_inventory(&mut sender, PAGED_FROM).await;
    drop(sender);
    let Some(after) = after else {
        match got {
            Err(TransferError::Inventory(why)) => {
                println!("shape {name}: refused: {why}");
                assert!(
                    why.contains("an inventory that takes more memory than PCTwin allows"),
                    "{name}: {why}"
                );
            }
            other => {
                panic!("{name}: expected the list to be refused at the memory limit: {other:?}")
            }
        }
        return;
    };
    let record = got.expect("a list within the limit is accepted");
    assert_eq!(
        record.items.len(),
        pages as usize * shape(base).per as usize
    );
    let approval = record
        .approve()
        .expect("a list with no people to place is approved");
    match after {
        After::Approve => {}
        After::Plan => {
            let plan = Allowance::from_record(&record, &approval, |i| {
                Some(LandingPlace::new("Documents", shown_path(&i.path)))
            })
            .expect("the plan follows the approval");
            let first = &record.items[0];
            assert_eq!(
                plan.check_place(first.id, "Documents", &shown_path(&first.path)),
                Ok(())
            );
        }
        After::Persist => {
            // A file of its own, removed when it is closed.
            let mut out = std::io::BufWriter::new(tempfile::tempfile().unwrap());
            record.write_json(&mut out).unwrap();
            std::io::Write::flush(&mut out).unwrap();
        }
    }
    println!("shape {name}: accepted and used");
}

/// An honest laptop: `n` files eight folders deep, ten-byte folder names, one person, as a scan
/// finds them.
fn honest(n: u64) -> Vec<pctwin_record::Item> {
    use pctwin_record::*;
    let laptop = LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap();
    let owner = Owner::Person {
        account_id: "S-1-5-21-1004336348-1177238915-682003330-1001".into(),
    };
    let place = Place {
        role: FolderRole::Documents,
        storage: Storage::SystemDrive,
    };
    (0..n)
        .map(|i| {
            let parts: Vec<ItemName> = (0..7)
                .map(|d| ItemName::from_text(&format!("folder{:04}", (i >> (d * 2)) % 37)))
                .chain(std::iter::once(ItemName::from_text(&format!(
                    "file{i:06}.docx"
                ))))
                .collect();
            let path = ItemPath::new(parts).unwrap();
            Item {
                id: ItemId::derive(&laptop, &owner, &place, &path),
                kind: ItemKind::File,
                owner: owner.clone(),
                place: place.clone(),
                path,
                size_bytes: 5,
                portability: Portability::Portable,
                managed_by: ManagedBy::Personal,
                download_bytes: None,
                inclusion: Inclusion::Included,
            }
        })
        .collect()
}

/// How many files one move takes, pinned: 680,000 files eight folders deep are sent; 700,000
/// are more than the new laptop may hold, and the old laptop refuses before sending anything.
#[test]
#[ignore = "run in release by scripts/inventory-memory.py"]
fn an_honest_laptop_of_680000_files_moves_and_one_of_700000_does_not() {
    let laptop = pctwin_record::LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap();
    let mut items = honest(700_000);
    assert!(pctwin_record::encode_inventory(&laptop, &items).is_err());
    items.truncate(680_000);
    let encoded = pctwin_record::encode_inventory(&laptop, &items).unwrap();
    let (_, read, _) = pctwin_record::decode_inventory(&encoded).unwrap();
    assert_eq!(read.len(), 680_000);
}
