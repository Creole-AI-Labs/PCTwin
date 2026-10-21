//! The scan inventory, version 2 (Engineering Plan 9.11, decided 16 October 2026): what the scan
//! found, in a small manifest and pages, so no laptop is too big to send and neither laptop has to
//! hold the whole list at once.
//!
//! - The **manifest** holds the format, the old laptop's ID, how many items and pages follow, and
//!   the owners and places, each stored once.
//! - Each **page** holds at most [`PAGE_ITEMS`] items and at most [`MAX_PAGE_BYTES`] bytes. A
//!   path is never repeated: a page first defines the folders its items need that no earlier page
//!   defined, each as a parent folder (always an earlier one) and a name, and each item names its
//!   folder and its own name.
//! - The form is a compact binary one written by hand (a kind byte where there is a choice,
//!   fixed-width big-endian numbers, lengths before bytes), as the transfer messages are, so no
//!   serialisation library enters the supply chain. Encoding is deterministic, and the reader
//!   accepts nothing PCTwin never writes (a folder defined twice or never used, an owner or a
//!   place nobody uses); where pages end may differ, which is why the fingerprint is taken over
//!   the bytes as they crossed rather than over a re-encoding.
//! - [`InventoryReader`] reads it as coming from a stranger: every length is checked before it is
//!   read, every index must be in range, a folder's parent must come before it, names follow the
//!   rules of [`ItemPath`], no item ID appears twice, pages come in order and add up to exactly
//!   what the manifest announced. Nothing panics on any input.
//! - **Memory is bounded by what is held, never by the bytes that crossed.** A path is at most
//!   [`MAX_PATH_DEPTH`] parts and [`MAX_PATH_BYTES`] bytes, as the safety gate allows, checked
//!   the moment a folder is defined. And before anything is copied, the reader works out what
//!   each folder and each item will take once read (every part of its path, every name, its
//!   owner and place) and refuses a page whose share passes [`MAX_PAGE_HELD`], or an inventory
//!   whose whole passes [`MAX_HELD`]. Every allocation is charged by its size through one
//!   function, more than the Windows, macOS and Linux allocators take, so the count is an upper
//!   bound on what is held, not an estimate. The writer ends a page early by the same count, so an
//!   honest laptop never meets the page limit. (A few hundred kilobytes of nested folders once
//!   read as gigabytes, since every item carries its whole path: found by a fresh-context review,
//!   20 October 2026. Capping path parts and name bytes alone still let a 0.5 MB page take
//!   147 MB, so the count is of memory itself.)
//! - The **fingerprint** is taken over the bytes exactly as they crossed (the manifest, then each
//!   page, each with its length), never over a laptop's own re-encoding, so two laptops on
//!   different PCTwin versions agree on it.

use std::collections::{HashMap, HashSet};

use sha2::{Digest, Sha256};

use crate::{
    CloudProvider, Fingerprint, FolderRole, Inclusion, Item, ItemId, ItemKind, ItemName, ItemPath,
    LaptopId, LeftOutReason, ManagedBy, Owner, Place, Portability, RawName, RecordError, Storage,
};

/// The inventory format this app writes and reads.
pub const INVENTORY_FORMAT: u32 = 2;
/// Most items in one page.
pub const PAGE_ITEMS: usize = 10_000;
/// Most bytes in one encoded page.
pub const MAX_PAGE_BYTES: usize = 8 * 1024 * 1024;
/// Most bytes in the manifest (it holds only counts, owners and places).
pub const MAX_MANIFEST_BYTES: usize = 1024 * 1024;
/// Longest name, in bytes (or UTF-16 units for an exact Windows name).
pub const MAX_NAME_LEN: usize = 4096;
/// Most parts in one path (its folders and its own name), as the safety gate allows.
pub const MAX_PATH_DEPTH: usize = 128;
/// Most bytes in one path, its names and the separators between them, as the safety gate allows.
pub const MAX_PATH_BYTES: usize = 4096;
/// Most memory one page's folders and items may take once read, as the reader counts it.
/// An ordinary page of 10,000 files takes about 10 MB.
pub const MAX_PAGE_HELD: u64 = 32 * 1024 * 1024;
/// Most memory a whole inventory may take once read, as the reader counts it. The count is an
/// upper bound on what the allocator takes (see [`alloc_held`]), so the reader never holds more
/// than this for the items and folders it has read, plus one page's bytes and its own fixed
/// running room. A CI step measures it on Windows, macOS and Linux
/// (`scripts/inventory-memory.py`). An honest laptop with files eight folders deep fits 680,000
/// files (700,000 are refused; measured 21 October 2026, pinned by a test run in CI). Received pages kept on disk
/// (Engineering Plan 9.11, step C) will lift the count of files without raising this.
pub const MAX_HELD: u64 = 1280 * 1024 * 1024;
/// What an item takes before its names: the item itself, three times over (a list that grows
/// as pages arrive holds up to twice its items, and three times for the moment it moves), and
/// its ID in the reader's set. Fixed numbers rather than this build's type sizes, so two PCTwin
/// versions count alike; a test checks they cover the real sizes. These costs, [`alloc_held`]
/// and the two limits are part of the format: the writer splits pages by them, so changing any
/// needs a new [`INVENTORY_FORMAT`].
const ITEM_COST: u64 = 448 + PLAN_ROOM;
/// Room left in each item's cost for its entry in the plan the new laptop makes once the list is
/// approved (where each file lands: its ID, size, a shared destination and a digest of the path,
/// in a map made with room for every item at once; a test in pctwin-transfer checks it fits).
pub const PLAN_ROOM: u64 = 192;
/// What each part of a path takes in the path's list, before its name's bytes.
const NAME_COST: u64 = 64;
/// What a folder takes in the reader before its name and key: the folder and its key, three
/// times over, as the lists and the set that hold them grow.
const FOLDER_COST: u64 = 384;
/// Most owners and most places in one inventory.
const MAX_TABLE: u32 = 65_536;
/// Longest text in an owner or a place (an account ID, a drive ID).
const MAX_LABEL: usize = 512;
/// Most pages.
const MAX_PAGES: u32 = 1 << 20;
/// "No folder": the item sits directly in its place.
const ROOT: u32 = u32::MAX;
const MAGIC: &[u8; 8] = b"PCTWINV2";
const FINGERPRINT_LABEL: &[u8] = b"pctwin/v2/inventory\0";

fn bad(why: &'static str) -> RecordError {
    RecordError::BadInventory(why)
}

// ---------------------------------------------------------------- writing

struct Writer(Vec<u8>);

impl Writer {
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_be_bytes());
    }
    fn u64(&mut self, v: u64) {
        self.0.extend_from_slice(&v.to_be_bytes());
    }
    fn bytes(&mut self, b: &[u8]) -> Result<(), RecordError> {
        self.u32(u32::try_from(b.len()).map_err(|_| bad("a text is too long"))?);
        self.0.extend_from_slice(b);
        Ok(())
    }
    fn name(&mut self, n: &ItemName) -> Result<(), RecordError> {
        match n.exact() {
            None => {
                if n.display().len() > MAX_NAME_LEN {
                    return Err(bad("a name is too long"));
                }
                self.u8(0);
                self.bytes(n.display().as_bytes())
            }
            Some(RawName::Unix(b)) => {
                if b.len() > MAX_NAME_LEN {
                    return Err(bad("a name is too long"));
                }
                self.u8(1);
                self.bytes(b)
            }
            Some(RawName::Windows(w)) => {
                if w.len() > MAX_NAME_LEN {
                    return Err(bad("a name is too long"));
                }
                self.u8(2);
                self.u32(u32::try_from(w.len()).map_err(|_| bad("a name is too long"))?);
                for unit in w {
                    self.0.extend_from_slice(&unit.to_be_bytes());
                }
                Ok(())
            }
        }
    }
    fn owner(&mut self, o: &Owner) -> Result<(), RecordError> {
        match o {
            Owner::Person { account_id } => {
                self.u8(0);
                label(account_id)?;
                self.bytes(account_id.as_bytes())
            }
            Owner::Shared => {
                self.u8(1);
                Ok(())
            }
            Owner::System => {
                self.u8(2);
                Ok(())
            }
        }
    }
    fn place(&mut self, p: &Place) -> Result<(), RecordError> {
        self.u8(role_code(p.role));
        match &p.storage {
            Storage::SystemDrive => self.u8(0),
            Storage::OtherDrive { drive } => {
                self.u8(1);
                label(drive)?;
                self.bytes(drive.as_bytes())?;
            }
            Storage::Cloud { provider } => {
                self.u8(2);
                self.u8(provider_code(*provider));
            }
        }
        Ok(())
    }
}

/// The length a path counts as: each name's bytes, and one separator between names.
fn path_bytes(parts: &[ItemName]) -> usize {
    parts.iter().map(|n| n.display().len()).sum::<usize>() + parts.len().saturating_sub(1)
}

fn path_too_long(parts: &[ItemName]) -> bool {
    parts.len() > MAX_PATH_DEPTH || path_bytes(parts) > MAX_PATH_BYTES
}

fn label(s: &str) -> Result<(), RecordError> {
    if s.len() > MAX_LABEL {
        return Err(bad("an owner or place is too long"));
    }
    Ok(())
}

// ---------------------------------------------------------------- what is held

/// What one allocation of `n` bytes may take from the allocator, counted on the side of too
/// much. Every separate allocation the reader makes (a name's text, its exact form, a copy of an
/// owner or a place, a folder's key, an item's path) is counted through this one function:
/// - a quarter more than asked: Windows keeps up to 17% more for blocks of 1 to 16 KB;
/// - or, past 1,008 bytes, the next multiple of 512 (macOS's blocks of that size), and below it
///   the next multiple of 16;
/// - and 32 bytes more for the allocator's own record (Linux's smallest block is 32 bytes).
fn alloc_held(n: u64) -> u64 {
    if n == 0 {
        return 0;
    }
    let quantum = if n > 1008 { 512 } else { 16 };
    (n + n / 4).max(n.div_ceil(quantum) * quantum) + 32
}

/// What a name holds: its text and, when kept, its exact form, each its own allocation.
fn name_heap(n: &ItemName) -> u64 {
    let exact = match n.exact() {
        None => 0,
        Some(RawName::Unix(b)) => alloc_held(b.len() as u64),
        Some(RawName::Windows(w)) => alloc_held(2 * w.len() as u64),
    };
    alloc_held(n.display().len() as u64) + exact
}

/// What each item's copy of this owner holds.
fn owner_heap(o: &Owner) -> u64 {
    match o {
        Owner::Person { account_id } => alloc_held(account_id.len() as u64),
        Owner::Shared | Owner::System => 0,
    }
}

/// What each item's copy of this place holds.
fn place_heap(p: &Place) -> u64 {
    match &p.storage {
        Storage::OtherDrive { drive } => alloc_held(drive.len() as u64),
        Storage::SystemDrive | Storage::Cloud { .. } => 0,
    }
}

/// The bytes `n` takes as written: its kind, its length, then its bytes or units.
fn written_len(n: &ItemName) -> usize {
    5 + match n.exact() {
        None => n.display().len(),
        Some(RawName::Unix(b)) => b.len(),
        Some(RawName::Windows(w)) => 2 * w.len(),
    }
}

/// What a folder takes in the reader: itself, its name, and its key (the name as written).
fn folder_held(name: &ItemName, written: usize) -> u64 {
    FOLDER_COST + name_heap(name) + alloc_held(written as u64)
}

/// What an item takes once read: itself, its owner and place, and every part of its path.
/// `chain` is what the names of its folders hold.
fn item_held(owner: u64, place: u64, depth: usize, chain: u64, name: &ItemName) -> u64 {
    ITEM_COST + owner + place + alloc_held(depth as u64 * NAME_COST) + chain + name_heap(name)
}

/// Adds `cost` to the page's and the whole inventory's count, refusing either past its limit.
fn hold(page: &mut u64, whole: &mut u64, cost: u64) -> Result<(), RecordError> {
    *page += cost;
    *whole += cost;
    if *page > MAX_PAGE_HELD {
        return Err(bad("a page that takes more memory than PCTwin allows"));
    }
    if *whole > MAX_HELD {
        return Err(bad(
            "an inventory that takes more memory than PCTwin allows",
        ));
    }
    Ok(())
}

/// The inventory as it is sent: the manifest's bytes and each page's bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedInventory {
    pub manifest: Vec<u8>,
    pub pages: Vec<Vec<u8>>,
}

impl EncodedInventory {
    /// The fingerprint of exactly these bytes (see the module notes).
    pub fn fingerprint(&self) -> Fingerprint {
        let mut h = Fingerprinter::new(&self.manifest);
        for page in &self.pages {
            h.page(page);
        }
        h.finish()
    }
}

struct Fingerprinter(Sha256);

impl Fingerprinter {
    fn new(manifest: &[u8]) -> Self {
        let mut h = Sha256::new();
        h.update(FINGERPRINT_LABEL);
        h.update((manifest.len() as u64).to_be_bytes());
        h.update(manifest);
        Self(h)
    }
    fn page(&mut self, page: &[u8]) {
        self.0.update((page.len() as u64).to_be_bytes());
        self.0.update(page);
    }
    fn finish(self) -> Fingerprint {
        Fingerprint(self.0.finalize().into())
    }
}

/// Writes the inventory of `items`, found on laptop `laptop`, in their order.
pub fn encode_inventory(
    laptop: &LaptopId,
    items: &[Item],
) -> Result<EncodedInventory, RecordError> {
    encode_counted(laptop, items).map(|(encoded, _)| encoded)
}

/// [`encode_inventory`], with what the reader will hold, in all, after each page.
fn encode_counted(
    laptop: &LaptopId,
    items: &[Item],
) -> Result<(EncodedInventory, Vec<u64>), RecordError> {
    // Owners and places, each stored once, in the order first met.
    let mut owners: Vec<&Owner> = Vec::new();
    let mut places: Vec<&Place> = Vec::new();
    let mut owner_ix: HashMap<Vec<u8>, u32> = HashMap::new();
    let mut place_ix: HashMap<Vec<u8>, u32> = HashMap::new();
    let mut item_owner = Vec::with_capacity(items.len());
    let mut item_place = Vec::with_capacity(items.len());
    for item in items {
        let mut w = Writer(Vec::new());
        w.owner(&item.owner)?;
        let next = u32::try_from(owners.len()).map_err(|_| bad("too many owners"))?;
        let o = *owner_ix.entry(w.0).or_insert_with(|| {
            owners.push(&item.owner);
            next
        });
        let mut w = Writer(Vec::new());
        w.place(&item.place)?;
        let next = u32::try_from(places.len()).map_err(|_| bad("too many places"))?;
        let p = *place_ix.entry(w.0).or_insert_with(|| {
            places.push(&item.place);
            next
        });
        if o >= MAX_TABLE || p >= MAX_TABLE {
            return Err(bad("too many owners or places"));
        }
        item_owner.push(o);
        item_place.push(p);
    }

    // Pages: folders defined when first needed, then the items.
    let mut folders = Folders {
        index: HashMap::new(),
        count: 0,
        held: 0,
    };
    let mut pages: Vec<Vec<u8>> = Vec::new();
    let mut held_after = Vec::new();
    let mut start = 0;
    while start < items.len() {
        let (page, used) = encode_page(
            pages.len(),
            &items[start..],
            &item_owner[start..],
            &item_place[start..],
            &mut folders,
        )?;
        pages.push(page);
        held_after.push(folders.held);
        start += used;
    }

    let mut m = Writer(Vec::new());
    m.0.extend_from_slice(MAGIC);
    m.u32(INVENTORY_FORMAT);
    m.0.extend_from_slice(&laptop.0);
    m.u64(items.len() as u64);
    m.u32(u32::try_from(pages.len()).map_err(|_| bad("too many pages"))?);
    m.u32(u32::try_from(owners.len()).map_err(|_| bad("too many owners"))?);
    for o in &owners {
        m.owner(o)?;
    }
    m.u32(u32::try_from(places.len()).map_err(|_| bad("too many places"))?);
    for p in &places {
        m.place(p)?;
    }
    if m.0.len() > MAX_MANIFEST_BYTES {
        return Err(bad("the manifest is too large"));
    }
    Ok((
        EncodedInventory {
            manifest: m.0,
            pages,
        },
        held_after,
    ))
}

/// A folder by its parent and its name as written.
type FolderKey = (u32, Vec<u8>);

/// The folders written so far: each by its parent and its name as written, with its index and
/// what the names of its path hold; how many; and what the reader will hold, in all, once it has
/// read the pages written so far.
struct Folders {
    index: HashMap<FolderKey, (u32, u64)>,
    count: u32,
    held: u64,
}

/// One page from the front of `items`: as many as fit in [`PAGE_ITEMS`], [`MAX_PAGE_BYTES`] and
/// [`MAX_PAGE_HELD`]. Returns the page and how many items it took.
fn encode_page(
    number: usize,
    items: &[Item],
    owners: &[u32],
    places: &[u32],
    known: &mut Folders,
) -> Result<(Vec<u8>, usize), RecordError> {
    let first_folder = known.count;
    let mut folders = Writer(Vec::new());
    let mut new_folders: u32 = 0;
    let mut body = Writer(Vec::new());
    let mut page_held: u64 = 0;
    let mut taken = 0;
    for (k, item) in items.iter().enumerate().take(PAGE_ITEMS) {
        if path_too_long(item.path.parts()) {
            return Err(bad("a path deeper or longer than PCTwin can move"));
        }
        // Work out this item on the side, so a page that would grow too large ends before it.
        let mut f = Writer(Vec::new());
        let mut added: Vec<(FolderKey, (u32, u64))> = Vec::new();
        let mut cost: u64 = 0;
        let parts = item.path.parts();
        let mut parent = ROOT;
        let mut chain: u64 = 0;
        for name in &parts[..parts.len() - 1] {
            let mut nw = Writer(Vec::new());
            nw.name(name)?;
            let key = (parent, nw.0.clone());
            let found = known
                .index
                .get(&key)
                .copied()
                .or_else(|| added.iter().find(|(k, _)| *k == key).map(|(_, v)| *v));
            (parent, chain) = match found {
                Some(v) => v,
                None => {
                    let i = known
                        .count
                        .checked_add(new_folders)
                        .and_then(|n| n.checked_add(u32::try_from(added.len()).ok()?))
                        .filter(|n| *n < ROOT)
                        .ok_or(bad("too many folders"))?;
                    let here = chain + name_heap(name);
                    cost += folder_held(name, nw.0.len());
                    f.u32(key.0);
                    f.0.extend_from_slice(&key.1);
                    added.push((key, (i, here)));
                    (i, here)
                }
            };
        }
        let name = &parts[parts.len() - 1];
        cost += item_held(
            owner_heap(&item.owner),
            place_heap(&item.place),
            parts.len(),
            chain,
            name,
        );
        let mut b = Writer(Vec::new());
        b.0.extend_from_slice(&item.id.0);
        b.u8(kind_code(item.kind));
        b.u32(owners[k]);
        b.u32(places[k]);
        b.u32(parent);
        b.name(name)?;
        b.u64(item.size_bytes);
        b.u8(portability_code(item.portability));
        b.u8(managed_code(item.managed_by));
        match item.download_bytes {
            None => b.u8(0),
            Some(n) => {
                b.u8(1);
                b.u64(n);
            }
        }
        b.u8(inclusion_code(item.inclusion));
        // Header: number, first folder, folder count, item count (4 numbers of 4 bytes).
        let size = 16 + folders.0.len() + f.0.len() + body.0.len() + b.0.len();
        if size > MAX_PAGE_BYTES || page_held + cost > MAX_PAGE_HELD {
            if taken == 0 {
                return Err(bad("one item is larger than a page"));
            }
            break;
        }
        if known.held + cost > MAX_HELD {
            return Err(bad(
                "an inventory that takes more memory than PCTwin allows",
            ));
        }
        page_held += cost;
        known.held += cost;
        folders.0.extend_from_slice(&f.0);
        new_folders += u32::try_from(added.len()).map_err(|_| bad("too many folders"))?;
        for (key, v) in added {
            known.index.insert(key, v);
        }
        body.0.extend_from_slice(&b.0);
        taken = k + 1;
    }
    known.count += new_folders;
    let mut page = Writer(Vec::with_capacity(16 + folders.0.len() + body.0.len()));
    page.u32(u32::try_from(number).map_err(|_| bad("too many pages"))?);
    page.u32(first_folder);
    page.u32(new_folders);
    page.u32(u32::try_from(taken).map_err(|_| bad("too many items"))?);
    page.0.extend_from_slice(&folders.0);
    page.0.extend_from_slice(&body.0);
    Ok((page.0, taken))
}

// ---------------------------------------------------------------- reading

struct Reader<'a> {
    rest: &'a [u8],
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], RecordError> {
        if self.rest.len() < n {
            return Err(bad("it ends too soon"));
        }
        let (a, b) = self.rest.split_at(n);
        self.rest = b;
        Ok(a)
    }
    fn u8(&mut self) -> Result<u8, RecordError> {
        Ok(self.take(1)?[0])
    }
    fn u32(&mut self) -> Result<u32, RecordError> {
        let b = self.take(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn u64(&mut self) -> Result<u64, RecordError> {
        let b = self.take(8)?;
        let mut a = [0u8; 8];
        a.copy_from_slice(b);
        Ok(u64::from_be_bytes(a))
    }
    fn bytes(&mut self, max: usize) -> Result<&'a [u8], RecordError> {
        let n = self.u32()? as usize;
        if n > max {
            return Err(bad("a text is too long"));
        }
        self.take(n)
    }
    fn text(&mut self, max: usize) -> Result<String, RecordError> {
        std::str::from_utf8(self.bytes(max)?)
            .map(str::to_string)
            .map_err(|_| bad("a text is not valid UTF-8"))
    }
    /// A name, made with nothing let go along the way: each of its allocations is one it keeps.
    fn name(&mut self) -> Result<ItemName, RecordError> {
        let name = match self.u8()? {
            0 => ItemName::from_text(
                std::str::from_utf8(self.bytes(MAX_NAME_LEN)?)
                    .map_err(|_| bad("a text is not valid UTF-8"))?,
            ),
            1 => {
                let raw = self.bytes(MAX_NAME_LEN)?;
                let n = ItemName::from_unix_bytes(raw);
                // The exact form is kept only for names that are not valid text.
                if n.exact().is_none() {
                    return Err(bad("an exact name that is valid text"));
                }
                n
            }
            2 => {
                let len = self.u32()? as usize;
                if len > MAX_NAME_LEN {
                    return Err(bad("a name is too long"));
                }
                let b = self.take(len * 2)?;
                let mut wide = Vec::with_capacity(len);
                wide.extend(b.as_chunks::<2>().0.iter().map(|c| u16::from_be_bytes(*c)));
                let n = ItemName::from_windows_units(wide);
                if n.exact().is_none() {
                    return Err(bad("an exact name that is valid text"));
                }
                n
            }
            _ => return Err(bad("an unknown kind of name")),
        };
        // The same rules as a path's parts.
        if crate::breaks_part_rules(name.display()) {
            return Err(bad("a name is empty, `.`, `..`, or holds `/` or a NUL"));
        }
        Ok(name)
    }
    fn owner(&mut self) -> Result<Owner, RecordError> {
        Ok(match self.u8()? {
            0 => Owner::Person {
                account_id: self.text(MAX_LABEL)?,
            },
            1 => Owner::Shared,
            2 => Owner::System,
            _ => return Err(bad("an unknown kind of owner")),
        })
    }
    fn place(&mut self) -> Result<Place, RecordError> {
        let role = role_of(self.u8()?)?;
        let storage = match self.u8()? {
            0 => Storage::SystemDrive,
            1 => Storage::OtherDrive {
                drive: self.text(MAX_LABEL)?,
            },
            2 => Storage::Cloud {
                provider: provider_of(self.u8()?)?,
            },
            _ => return Err(bad("an unknown kind of place")),
        };
        Ok(Place { role, storage })
    }
}

/// Reads an inventory page by page, checking everything as it goes (see the module notes).
pub struct InventoryReader {
    laptop: LaptopId,
    items: u64,
    pages: u32,
    owners: Vec<Owner>,
    places: Vec<Place>,
    owner_used: Vec<bool>,
    place_used: Vec<bool>,
    /// What each item's copy of an owner or a place holds.
    owner_heap: Vec<u64>,
    place_heap: Vec<u64>,
    folders: Vec<Folder>,
    /// Each folder once: its parent and its name as written.
    folder_keys: HashSet<(u32, Vec<u8>)>,
    next_page: u32,
    items_read: u64,
    /// What the folders and items read so far hold, as [`MAX_HELD`] counts it.
    held: u64,
    ids: HashSet<ItemId>,
    hash: Fingerprinter,
}

/// A folder defined by a page.
struct Folder {
    parent: u32,
    name: ItemName,
    /// Parts in its path (itself included), the path's bytes, and what the path's names hold.
    depth: usize,
    bytes: usize,
    chain: u64,
    used: bool,
}

impl InventoryReader {
    /// Reads the manifest.
    pub fn start(manifest: &[u8]) -> Result<Self, RecordError> {
        if manifest.len() > MAX_MANIFEST_BYTES {
            return Err(bad("the manifest is too large"));
        }
        let mut r = Reader { rest: manifest };
        if r.take(MAGIC.len())? != MAGIC {
            return Err(bad("this is not a PCTwin inventory"));
        }
        let format = r.u32()?;
        if format > INVENTORY_FORMAT {
            return Err(RecordError::NewerFormat { found: format });
        }
        if format != INVENTORY_FORMAT {
            return Err(bad("an inventory format this app does not read"));
        }
        let mut id = [0u8; 16];
        id.copy_from_slice(r.take(16)?);
        let items = r.u64()?;
        let pages = r.u32()?;
        if pages > MAX_PAGES || items > u64::from(pages) * PAGE_ITEMS as u64 {
            return Err(bad("more items or pages than an inventory can hold"));
        }
        if (items == 0) != (pages == 0) {
            return Err(bad("pages without items, or items without pages"));
        }
        let owners_n = r.u32()?;
        if owners_n > MAX_TABLE {
            return Err(bad("too many owners"));
        }
        let mut owners = Vec::new();
        for _ in 0..owners_n {
            owners.push(r.owner()?);
        }
        let places_n = r.u32()?;
        if places_n > MAX_TABLE {
            return Err(bad("too many places"));
        }
        let mut places = Vec::new();
        for _ in 0..places_n {
            places.push(r.place()?);
        }
        if !r.rest.is_empty() {
            return Err(bad("the manifest has bytes after its end"));
        }
        // Stored once: a table with the same entry twice is not one PCTwin writes.
        let mut seen = HashSet::new();
        for o in &owners {
            let mut w = Writer(Vec::new());
            w.owner(o)?;
            if !seen.insert(w.0) {
                return Err(bad("an owner is listed twice"));
            }
        }
        seen.clear();
        for p in &places {
            let mut w = Writer(Vec::new());
            w.place(p)?;
            if !seen.insert(w.0) {
                return Err(bad("a place is listed twice"));
            }
        }
        Ok(Self {
            laptop: LaptopId(id),
            items,
            pages,
            owner_used: vec![false; owners.len()],
            place_used: vec![false; places.len()],
            owner_heap: owners.iter().map(owner_heap).collect(),
            place_heap: places.iter().map(place_heap).collect(),
            owners,
            places,
            folders: Vec::new(),
            folder_keys: HashSet::new(),
            next_page: 0,
            items_read: 0,
            held: 0,
            ids: HashSet::new(),
            hash: Fingerprinter::new(manifest),
        })
    }

    /// The old laptop the inventory is from.
    pub fn laptop(&self) -> LaptopId {
        self.laptop
    }

    /// Items the manifest announces.
    pub fn item_count(&self) -> u64 {
        self.items
    }

    /// Pages the manifest announces.
    pub fn page_count(&self) -> u32 {
        self.pages
    }

    /// Reads the next page and returns its items.
    pub fn page(&mut self, bytes: &[u8]) -> Result<Vec<Item>, RecordError> {
        let mut items = Vec::new();
        self.page_into(bytes, &mut items)?;
        Ok(items)
    }

    /// Reads the next page, adding its items to the end of `items`, so a laptop gathering the
    /// whole list makes no list per page to let go of (on a refusal, `items` is as it was).
    pub fn page_into(&mut self, bytes: &[u8], items: &mut Vec<Item>) -> Result<(), RecordError> {
        let before = items.len();
        let read = self.read_page(bytes, items);
        if read.is_err() {
            items.truncate(before);
        }
        read
    }

    fn read_page(&mut self, bytes: &[u8], items: &mut Vec<Item>) -> Result<(), RecordError> {
        if self.next_page >= self.pages {
            return Err(bad("more pages than announced"));
        }
        if bytes.len() > MAX_PAGE_BYTES {
            return Err(bad("a page is too large"));
        }
        let mut r = Reader { rest: bytes };
        if r.u32()? != self.next_page {
            return Err(bad("a page out of order"));
        }
        if r.u32()? as usize != self.folders.len() {
            return Err(bad("a page's folders do not follow the earlier ones"));
        }
        let new_folders = r.u32()?;
        let count = r.u32()? as usize;
        if count == 0 || count > PAGE_ITEMS {
            return Err(bad("a page with no items or too many"));
        }
        let last = self.next_page + 1 == self.pages;
        let left = self.items - self.items_read;
        if count as u64 > left || (last && count as u64 != left) {
            return Err(bad("the pages do not add up to the items announced"));
        }
        let first_new = self.folders.len();
        let mut page_held: u64 = 0;
        for _ in 0..new_folders {
            let parent = r.u32()?;
            let name = r.name()?;
            let (depth, bytes, chain) = if parent == ROOT {
                (1, name.display().len(), name_heap(&name))
            } else {
                let p = self
                    .folders
                    .get_mut(parent as usize)
                    .ok_or(bad("a folder's parent does not come before it"))?;
                p.used = true;
                (
                    p.depth + 1,
                    p.bytes + 1 + name.display().len(),
                    p.chain + name_heap(&name),
                )
            };
            // Room is left for at least the item's own name.
            if depth >= MAX_PATH_DEPTH || bytes >= MAX_PATH_BYTES {
                return Err(bad("a folder deeper or longer than a path may be"));
            }
            // Made at exactly its size, as it is counted.
            let mut key = Writer(Vec::with_capacity(written_len(&name)));
            key.name(&name)?;
            hold(
                &mut page_held,
                &mut self.held,
                folder_held(&name, key.0.len()),
            )?;
            if !self.folder_keys.insert((parent, key.0)) {
                return Err(bad("a folder is defined twice"));
            }
            if self.folders.len() >= ROOT as usize {
                return Err(bad("too many folders"));
            }
            self.folders.push(Folder {
                parent,
                name,
                depth,
                bytes,
                chain,
                used: false,
            });
        }
        items.reserve(count);
        for _ in 0..count {
            let mut id = [0u8; 16];
            id.copy_from_slice(r.take(16)?);
            let id = ItemId(id);
            let kind = kind_of(r.u8()?)?;
            let o = r.u32()? as usize;
            let owner = self
                .owners
                .get(o)
                .ok_or(bad("an owner index out of range"))?
                .clone();
            self.owner_used[o] = true;
            let p = r.u32()? as usize;
            let place = self
                .places
                .get(p)
                .ok_or(bad("a place index out of range"))?
                .clone();
            self.place_used[p] = true;
            let folder = r.u32()?;
            let name = r.name()?;
            let size_bytes = r.u64()?;
            let portability = portability_of(r.u8()?)?;
            let managed_by = managed_of(r.u8()?)?;
            let download_bytes = match r.u8()? {
                0 => None,
                1 => Some(r.u64()?),
                _ => return Err(bad("an unknown download size marker")),
            };
            let inclusion = inclusion_of(r.u8()?)?;
            let path = self.path_of(folder, name, o, p, &mut page_held)?;
            if !self.ids.insert(id) {
                return Err(RecordError::DuplicateId);
            }
            items.push(Item {
                id,
                kind,
                owner,
                place,
                path,
                size_bytes,
                portability,
                managed_by,
                download_bytes,
                inclusion,
            });
        }
        if !r.rest.is_empty() {
            return Err(bad("a page has bytes after its end"));
        }
        // A page defines only the folders its own items need.
        if self.folders[first_new..].iter().any(|f| !f.used) {
            return Err(bad("a page defines a folder nothing in it uses"));
        }
        self.hash.page(bytes);
        self.next_page += 1;
        self.items_read += count as u64;
        Ok(())
    }

    /// The fingerprint, once every page was read.
    pub fn finish(self) -> Result<Fingerprint, RecordError> {
        if self.next_page != self.pages || self.items_read != self.items {
            return Err(bad("pages are missing"));
        }
        if self.owner_used.contains(&false) || self.place_used.contains(&false) {
            return Err(bad("an owner or a place nothing uses"));
        }
        Ok(self.hash.finish())
    }

    /// The path of an item named `name` in `folder`, owned by owner `o` in place `p`, once what
    /// it will hold is counted (before anything is copied).
    fn path_of(
        &mut self,
        folder: u32,
        name: ItemName,
        o: usize,
        p: usize,
        page_held: &mut u64,
    ) -> Result<ItemPath, RecordError> {
        let (depth, bytes, chain) = if folder == ROOT {
            (1, name.display().len(), 0)
        } else {
            let f = self
                .folders
                .get_mut(folder as usize)
                .ok_or(bad("a folder index out of range"))?;
            f.used = true;
            (f.depth + 1, f.bytes + 1 + name.display().len(), f.chain)
        };
        if depth > MAX_PATH_DEPTH || bytes > MAX_PATH_BYTES {
            return Err(bad("a path deeper or longer than PCTwin can move"));
        }
        let cost = item_held(self.owner_heap[o], self.place_heap[p], depth, chain, &name);
        hold(page_held, &mut self.held, cost)?;
        let mut parts = Vec::with_capacity(depth);
        parts.push(name);
        let mut at = folder;
        // Parents always come before their folders, and depth is bounded, so this walk ends.
        while at != ROOT {
            let f = &self.folders[at as usize];
            parts.push(f.name.clone());
            at = f.parent;
        }
        parts.reverse();
        ItemPath::new(parts).map_err(|_| bad("a path that breaks the rules"))
    }
}

/// Reads a whole inventory at once: its laptop, items and fingerprint.
pub fn decode_inventory(
    encoded: &EncodedInventory,
) -> Result<(LaptopId, Vec<Item>, Fingerprint), RecordError> {
    let mut reader = InventoryReader::start(&encoded.manifest)?;
    let mut items = Vec::new();
    for page in &encoded.pages {
        items.extend(reader.page(page)?);
    }
    let laptop = reader.laptop();
    Ok((laptop, items, reader.finish()?))
}

// ---------------------------------------------------------------- the codes

macro_rules! codes {
    ($code:ident, $of:ident, $ty:ty, $what:literal, [$($v:path = $n:literal),* $(,)?]) => {
        fn $code(v: $ty) -> u8 {
            match v {
                $($v => $n,)*
            }
        }
        fn $of(n: u8) -> Result<$ty, RecordError> {
            match n {
                $($n => Ok($v),)*
                _ => Err(bad($what)),
            }
        }
    };
}

codes!(
    kind_code,
    kind_of,
    ItemKind,
    "an unknown kind of item",
    [
        ItemKind::File = 0,
        ItemKind::Folder = 1,
        ItemKind::App = 2,
        ItemKind::Setting = 3,
    ]
);
codes!(
    role_code,
    role_of,
    FolderRole,
    "an unknown folder role",
    [
        FolderRole::Home = 0,
        FolderRole::Desktop = 1,
        FolderRole::Documents = 2,
        FolderRole::Downloads = 3,
        FolderRole::Pictures = 4,
        FolderRole::Music = 5,
        FolderRole::Videos = 6,
        FolderRole::Public = 7,
        FolderRole::AppData = 8,
        FolderRole::Other = 9,
    ]
);
codes!(
    provider_code,
    provider_of,
    CloudProvider,
    "an unknown cloud service",
    [
        CloudProvider::OneDrive = 0,
        CloudProvider::ICloud = 1,
        CloudProvider::Dropbox = 2,
        CloudProvider::GoogleDrive = 3,
        CloudProvider::Other = 4,
    ]
);
codes!(
    portability_code,
    portability_of,
    Portability,
    "an unknown portability",
    [
        Portability::Portable = 0,
        Portability::Rebuildable = 1,
        Portability::DeviceBound = 2,
        Portability::LimitedActivations = 3,
    ]
);
codes!(
    managed_code,
    managed_of,
    ManagedBy,
    "an unknown manager",
    [ManagedBy::Personal = 0, ManagedBy::Organisation = 1,]
);

fn inclusion_code(i: Inclusion) -> u8 {
    match i {
        Inclusion::Included => 0,
        Inclusion::LeftOut { reason } => 1 + reason_code(reason),
    }
}

fn inclusion_of(n: u8) -> Result<Inclusion, RecordError> {
    match n {
        0 => Ok(Inclusion::Included),
        n => Ok(Inclusion::LeftOut {
            reason: reason_of(n - 1)?,
        }),
    }
}

codes!(
    reason_code,
    reason_of,
    LeftOutReason,
    "an unknown reason",
    [
        LeftOutReason::ChosenByUser = 0,
        LeftOutReason::WorkManaged = 1,
        LeftOutReason::DeviceBound = 2,
        LeftOutReason::Unreadable = 3,
        LeftOutReason::CloudOnly = 4,
        LeftOutReason::System = 5,
    ]
);

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    /// The fixed costs cover what this build really holds: an item in a growing list and its ID
    /// in a set (a set keeps at most about twice its entries' room), a path part, and a folder
    /// with its key.
    #[test]
    fn the_fixed_costs_cover_the_real_sizes() {
        assert!(
            3 * size_of::<Item>() + 2 * size_of::<ItemId>() <= (ITEM_COST - PLAN_ROOM) as usize
        );
        assert!(size_of::<ItemName>() <= NAME_COST as usize);
        assert!(3 * (size_of::<Folder>() + size_of::<FolderKey>() + 1) <= FOLDER_COST as usize);
    }

    /// The allocation charge: never less than Windows (measured: 8,000 bytes took 9,366 and
    /// 10,000 took 11,678), macOS (blocks past 1,008 bytes in multiples of 512) or Linux (32
    /// bytes at the least) take.
    #[test]
    fn an_allocation_is_charged_at_least_what_any_system_takes() {
        assert_eq!(alloc_held(0), 0);
        assert_eq!(alloc_held(1), 48);
        assert_eq!(alloc_held(1008), 1292);
        assert_eq!(alloc_held(1025), 1568);
        assert!(alloc_held(8000) >= 9366);
        assert!(alloc_held(10_000) >= 11_678);
        for n in 1..20_000u64 {
            let got = alloc_held(n);
            assert!(got > n * 117 / 100 && got >= 32, "{n}");
            if n > 1008 {
                assert!(got >= n.div_ceil(512) * 512, "{n}");
            }
        }
    }

    /// A folder's key is made at exactly the size it is counted at.
    #[test]
    fn a_name_as_written_is_as_long_as_counted() {
        for n in [
            ItemName::from_text("report.docx"),
            ItemName::from_unix_bytes(&[b'a', 0xFF]),
            ItemName::from_windows_wide(&[0xD800, 0x61, 0x62]),
        ] {
            let mut w = Writer(Vec::with_capacity(written_len(&n)));
            w.name(&n).unwrap();
            assert_eq!(w.0.len(), written_len(&n));
            assert_eq!(w.0.capacity(), w.0.len());
        }
    }

    /// The reader keeps every folder's key at exactly the size it counts, and every name at
    /// exactly its length.
    #[test]
    fn what_the_reader_keeps_is_exactly_the_size_it_counts() {
        let laptop = LaptopId([7; 16]);
        let place = Place {
            role: FolderRole::Documents,
            storage: Storage::SystemDrive,
        };
        let items: Vec<Item> = (0..3u8)
            .map(|i| {
                let path = ItemPath::new(vec![
                    ItemName::from_text("Projects and plans"),
                    ItemName::from_unix_bytes(&[b'x', 0xFF, b'a' + i]),
                    ItemName::from_windows_wide(&[0xD800, 0x61 + u16::from(i)]),
                    ItemName::from_text(&format!("file {i}.txt")),
                ])
                .unwrap();
                Item {
                    id: ItemId::derive(&laptop, &Owner::Shared, &place, &path),
                    kind: ItemKind::File,
                    owner: Owner::Shared,
                    place: place.clone(),
                    path,
                    size_bytes: 1,
                    portability: Portability::Portable,
                    managed_by: ManagedBy::Personal,
                    download_bytes: None,
                    inclusion: Inclusion::Included,
                }
            })
            .collect();
        let enc = encode_inventory(&laptop, &items).unwrap();
        let mut reader = InventoryReader::start(&enc.manifest).unwrap();
        let read = reader.page(&enc.pages[0]).unwrap();
        assert!(reader.folder_keys.len() >= 5);
        for (_, key) in &reader.folder_keys {
            assert_eq!(key.capacity(), key.len());
        }
        for item in &read {
            for part in item.path.parts() {
                assert_eq!(part.text.capacity(), part.text.len());
            }
        }
    }

    /// The reader counts exactly what the writer counted, page by page, for owners and places
    /// with long labels, exact Windows and Mac/Linux names and plain names (each past the
    /// 16-byte quantum), and folders defined on one page and used on later ones.
    #[test]
    fn the_reader_counts_exactly_what_the_writer_counted_page_by_page() {
        let laptop = LaptopId([9; 16]);
        let mut items = Vec::new();
        for i in 0..25_000u32 {
            let owner = if i % 3 == 0 {
                Owner::Shared
            } else {
                Owner::Person {
                    account_id: format!("S-1-5-21-1004336348-1177238915-{}", i % 7),
                }
            };
            let place = Place {
                role: FolderRole::Documents,
                storage: if i % 2 == 0 {
                    Storage::SystemDrive
                } else {
                    Storage::OtherDrive {
                        drive: format!("removable-drive-serial-{}", i % 5),
                    }
                },
            };
            let mut wide = vec![0xD800];
            wide.extend(std::iter::repeat_n(0x61 + (i % 11) as u16, 30));
            let mut raw = vec![0xFF];
            raw.extend(std::iter::repeat_n(b'a' + (i % 13) as u8, 40));
            let folder = match i % 3 {
                0 => ItemName::from_windows_wide(&wide),
                1 => ItemName::from_unix_bytes(&raw),
                _ => ItemName::from_text(&format!("Projects and plans {}", i % 17)),
            };
            let path = ItemPath::new(vec![
                ItemName::from_text(&format!("Folder number {}", i / 500)),
                folder,
                ItemName::from_text(&format!("a file with a long name {i}.txt")),
            ])
            .unwrap();
            items.push(Item {
                id: ItemId::derive(&laptop, &owner, &place, &path),
                kind: ItemKind::File,
                owner,
                place,
                path,
                size_bytes: 1,
                portability: Portability::Portable,
                managed_by: ManagedBy::Personal,
                download_bytes: None,
                inclusion: Inclusion::Included,
            });
        }
        let (enc, held_after) = encode_counted(&laptop, &items).unwrap();
        assert!(enc.pages.len() >= 3);
        let mut reader = InventoryReader::start(&enc.manifest).unwrap();
        for (page, written) in enc.pages.iter().zip(&held_after) {
            drop(reader.page(page).unwrap());
            assert_eq!(reader.held, *written);
        }
    }

    /// Every separate allocation is charged, each through the one function.
    #[test]
    fn every_allocation_of_a_name_an_owner_a_place_a_folder_and_a_path_is_charged() {
        let text = ItemName::from_text("report.docx");
        assert_eq!(name_heap(&text), alloc_held(11));
        let unix = ItemName::from_unix_bytes(&[b'a', 0xFF, b'b']);
        assert_eq!(name_heap(&unix), alloc_held(5) + alloc_held(3));
        let wide = ItemName::from_windows_wide(&[0xD800, 0x61]);
        assert_eq!(name_heap(&wide), alloc_held(4) + alloc_held(4));
        // Past the 16-byte quantum, so a charge of half shows: 40 units are 80 bytes exactly,
        // and their text 3 + 39 bytes.
        let mut units = vec![0xD800];
        units.extend(std::iter::repeat_n(0x61, 39));
        let long = ItemName::from_windows_wide(&units);
        assert_eq!(name_heap(&long), alloc_held(42) + alloc_held(80));
        let mut bytes = vec![0xFF];
        bytes.extend(std::iter::repeat_n(b'b', 59));
        let long = ItemName::from_unix_bytes(&bytes);
        assert_eq!(name_heap(&long), alloc_held(62) + alloc_held(60));
        let person = Owner::Person {
            account_id: "S-1-5-21".into(),
        };
        assert_eq!(owner_heap(&person), alloc_held(8));
        assert_eq!(owner_heap(&Owner::Shared), 0);
        let drive = Place {
            role: FolderRole::Music,
            storage: Storage::OtherDrive {
                drive: "USB-7".into(),
            },
        };
        assert_eq!(place_heap(&drive), alloc_held(5));
        assert_eq!(
            folder_held(&wide, 13),
            FOLDER_COST + name_heap(&wide) + alloc_held(13)
        );
        assert_eq!(
            item_held(7, 9, 3, 100, &text),
            ITEM_COST + 7 + 9 + alloc_held(3 * NAME_COST) + 100 + name_heap(&text)
        );
        assert_eq!(ITEM_COST, 640);
    }

    /// Each limit is met on its own: a page past its share, and pages each within theirs that
    /// add up past the whole.
    #[test]
    fn what_is_held_is_refused_past_either_limit() {
        let (mut page, mut whole) = (0, 0);
        hold(&mut page, &mut whole, MAX_PAGE_HELD).unwrap();
        assert!(hold(&mut page, &mut whole, 1).is_err());
        let mut whole = 0;
        let mut refused = None;
        for n in 0..=MAX_HELD / MAX_PAGE_HELD {
            let mut page = 0;
            if hold(&mut page, &mut whole, MAX_PAGE_HELD).is_err() {
                refused = Some(n);
                break;
            }
        }
        assert_eq!(refused, Some(MAX_HELD / MAX_PAGE_HELD));
    }

    /// The writer refuses, before sending anything, an inventory the reader would refuse.
    #[test]
    fn the_writer_stops_where_the_reader_would() {
        let laptop = LaptopId([7; 16]);
        let owner = Owner::Shared;
        let place = Place {
            role: FolderRole::Documents,
            storage: Storage::SystemDrive,
        };
        let path = ItemPath::new(vec![ItemName::from_text("a"), ItemName::from_text("b")]).unwrap();
        let item = Item {
            id: ItemId::derive(&laptop, &owner, &place, &path),
            kind: ItemKind::File,
            owner,
            place,
            path,
            size_bytes: 1,
            portability: Portability::Portable,
            managed_by: ManagedBy::Personal,
            download_bytes: None,
            inclusion: Inclusion::Included,
        };
        // One folder "a" and the item: what the reader would hold for them.
        let cost = folder_held(&ItemName::from_text("a"), 6)
            + item_held(
                0,
                0,
                2,
                name_heap(&ItemName::from_text("a")),
                &ItemName::from_text("b"),
            );
        for (before, fits) in [(MAX_HELD - cost, true), (MAX_HELD - cost + 1, false)] {
            let mut known = Folders {
                index: HashMap::new(),
                count: 0,
                held: before,
            };
            let got = encode_page(0, std::slice::from_ref(&item), &[0], &[0], &mut known);
            assert_eq!(got.is_ok(), fits, "{got:?}");
        }
    }
}
