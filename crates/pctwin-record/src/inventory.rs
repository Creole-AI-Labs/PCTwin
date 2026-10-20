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
//!   serialisation library enters the supply chain. There is exactly one encoding of each
//!   inventory, so encoding what was decoded gives the same bytes.
//! - [`InventoryReader`] reads it as coming from a stranger: every length is checked before it is
//!   read, every index must be in range, a folder's parent must come before it, names follow the
//!   rules of [`ItemPath`], no item ID appears twice, pages come in order and add up to exactly
//!   what the manifest announced. Nothing panics on any input.
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

fn label(s: &str) -> Result<(), RecordError> {
    if s.len() > MAX_LABEL {
        return Err(bad("an owner or place is too long"));
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
    let mut folder_ix: HashMap<(u32, Vec<u8>), u32> = HashMap::new();
    let mut folder_count: u32 = 0;
    let mut pages: Vec<Vec<u8>> = Vec::new();
    let mut start = 0;
    while start < items.len() {
        let (page, used) = encode_page(
            pages.len(),
            &items[start..],
            &item_owner[start..],
            &item_place[start..],
            &mut folder_ix,
            &mut folder_count,
        )?;
        pages.push(page);
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
    Ok(EncodedInventory {
        manifest: m.0,
        pages,
    })
}

/// One page from the front of `items`: as many as fit in [`PAGE_ITEMS`] and [`MAX_PAGE_BYTES`].
/// Returns the page and how many items it took.
fn encode_page(
    number: usize,
    items: &[Item],
    owners: &[u32],
    places: &[u32],
    folder_ix: &mut HashMap<(u32, Vec<u8>), u32>,
    folder_count: &mut u32,
) -> Result<(Vec<u8>, usize), RecordError> {
    let first_folder = *folder_count;
    let mut folders = Writer(Vec::new());
    let mut new_folders: u32 = 0;
    let mut body = Writer(Vec::new());
    let mut taken = 0;
    for (k, item) in items.iter().enumerate().take(PAGE_ITEMS) {
        // Work out this item on the side, so a page that would grow too large ends before it.
        let mut f = Writer(Vec::new());
        let mut added: Vec<((u32, Vec<u8>), u32)> = Vec::new();
        let parts = item.path.parts();
        let mut parent = ROOT;
        for name in &parts[..parts.len() - 1] {
            let mut nw = Writer(Vec::new());
            nw.name(name)?;
            let key = (parent, nw.0.clone());
            let known = folder_ix
                .get(&key)
                .copied()
                .or_else(|| added.iter().find(|(k, _)| *k == key).map(|(_, i)| *i));
            parent = match known {
                Some(i) => i,
                None => {
                    let i = folder_count
                        .checked_add(new_folders)
                        .and_then(|n| n.checked_add(u32::try_from(added.len()).ok()?))
                        .filter(|n| *n < ROOT)
                        .ok_or(bad("too many folders"))?;
                    f.u32(key.0);
                    f.0.extend_from_slice(&key.1);
                    added.push((key, i));
                    i
                }
            };
        }
        let mut b = Writer(Vec::new());
        b.0.extend_from_slice(&item.id.0);
        b.u8(kind_code(item.kind));
        b.u32(owners[k]);
        b.u32(places[k]);
        b.u32(parent);
        b.name(&parts[parts.len() - 1])?;
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
        if size > MAX_PAGE_BYTES {
            if taken == 0 {
                return Err(bad("one item is larger than a page"));
            }
            break;
        }
        folders.0.extend_from_slice(&f.0);
        new_folders += u32::try_from(added.len()).map_err(|_| bad("too many folders"))?;
        for (key, i) in added {
            folder_ix.insert(key, i);
        }
        body.0.extend_from_slice(&b.0);
        taken = k + 1;
    }
    *folder_count += new_folders;
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
    fn name(&mut self) -> Result<ItemName, RecordError> {
        let name = match self.u8()? {
            0 => ItemName::from_text(&self.text(MAX_NAME_LEN)?),
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
                let wide: Vec<u16> = b
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|c| u16::from_be_bytes(*c))
                    .collect();
                let n = ItemName::from_windows_wide(&wide);
                if n.exact().is_none() {
                    return Err(bad("an exact name that is valid text"));
                }
                n
            }
            _ => return Err(bad("an unknown kind of name")),
        };
        // The same rules as a path's parts.
        ItemPath::new(vec![name.clone()])
            .map_err(|_| bad("a name is empty, `.`, `..`, or holds `/` or a NUL"))?;
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
    /// Every folder so far: its parent (or [`ROOT`]) and its name.
    folders: Vec<(u32, ItemName)>,
    next_page: u32,
    items_read: u64,
    ids: HashSet<ItemId>,
    hash: Fingerprinter,
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
        for (i, o) in owners.iter().enumerate() {
            if owners[..i].contains(o) {
                return Err(bad("an owner is listed twice"));
            }
        }
        for (i, p) in places.iter().enumerate() {
            if places[..i].contains(p) {
                return Err(bad("a place is listed twice"));
            }
        }
        Ok(Self {
            laptop: LaptopId(id),
            items,
            pages,
            owners,
            places,
            folders: Vec::new(),
            next_page: 0,
            items_read: 0,
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
        for _ in 0..new_folders {
            let parent = r.u32()?;
            if parent != ROOT && parent as usize >= self.folders.len() {
                return Err(bad("a folder's parent does not come before it"));
            }
            let name = r.name()?;
            if self.folders.len() >= ROOT as usize {
                return Err(bad("too many folders"));
            }
            self.folders.push((parent, name));
        }
        let mut items = Vec::with_capacity(count);
        for _ in 0..count {
            let mut id = [0u8; 16];
            id.copy_from_slice(r.take(16)?);
            let id = ItemId(id);
            let kind = kind_of(r.u8()?)?;
            let owner = self
                .owners
                .get(r.u32()? as usize)
                .ok_or(bad("an owner index out of range"))?
                .clone();
            let place = self
                .places
                .get(r.u32()? as usize)
                .ok_or(bad("a place index out of range"))?
                .clone();
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
            let path = self.path_of(folder, name)?;
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
        self.hash.page(bytes);
        self.next_page += 1;
        self.items_read += count as u64;
        Ok(items)
    }

    /// The fingerprint, once every page was read.
    pub fn finish(self) -> Result<Fingerprint, RecordError> {
        if self.next_page != self.pages || self.items_read != self.items {
            return Err(bad("pages are missing"));
        }
        Ok(self.hash.finish())
    }

    fn path_of(&self, folder: u32, name: ItemName) -> Result<ItemPath, RecordError> {
        let mut parts = vec![name];
        let mut at = folder;
        // Parents always come before their folders, so this walk ends.
        while at != ROOT {
            let (parent, n) = self
                .folders
                .get(at as usize)
                .ok_or(bad("a folder index out of range"))?;
            parts.push(n.clone());
            at = *parent;
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
