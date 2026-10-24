//! The shared move record (product notes "How PCTwin understands two real laptops"; the plan).
//!
//! - Every item found on the old laptop gets one [`Item`]: a permanent [`ItemId`], where it really
//!   lives ([`Place`]), whose it is ([`Owner`]), whether it can move ([`Portability`]), who manages
//!   it, its download size, and, if it is left out, why ([`Inclusion`]). Items that are left out
//!   are recorded too, so the report, undo, resume and "bring me up to date" can point at them.
//! - Names are kept exactly as the disk had them ([`ItemName`]): as text when they are valid text,
//!   and as the exact bytes as well when they are not, so nothing is lost on the way back.
//! - One [`Mapping`] says where each person's things and the shared things go. Two people never
//!   land in one account.
//! - The record is kept as numbered revisions, each linked to the one before by its
//!   [`Fingerprint`]. A plan [`Approval`] holds for exactly one revision: any change, with or
//!   without a new revision number, needs a new approval.
//! - Every saved record carries [`FORMAT`]. A record from a newer app is refused safely
//!   (engineering notes), and a loaded record is checked like a new one.

use std::collections::{BTreeMap, HashSet};

use pctwin_types::format::{Format, FormatError};
use serde::{Deserialize, Serialize};
pub mod inventory;

pub use inventory::{
    EncodedInventory, INVENTORY_FORMAT, InventoryReader, MAX_HELD, MAX_PAGE_BYTES, MAX_PAGE_HELD,
    MAX_PATH_BYTES, MAX_PATH_DEPTH, PAGE_ITEMS, decode_inventory, encode_inventory,
    encode_inventory_ordered, fits_one_move,
};
use sha2::{Digest, Sha256};

/// The record format this app writes and reads.
pub const FORMAT: u32 = 1;

/// The record's place in the shared format rule (no older formats yet).
pub const RECORD_FORMAT: Format = Format {
    name: "move record",
    current: FORMAT,
    readable: &[],
};

/// Why a record was refused.
#[derive(Debug, thiserror::Error)]
pub enum RecordError {
    #[error("this record was made by a newer version of PCTwin (format {found})")]
    NewerFormat { found: u32 },
    #[error("this is not a PCTwin record this app can read: {0}")]
    BadFormat(String),
    #[error("a name is empty, `.`, `..`, or contains `/` or a NUL")]
    BadName,
    #[error("two items have the same ID")]
    DuplicateId,
    #[error("something tied to the old laptop was included in the move")]
    DeviceBoundIncluded,
    #[error("two people would land in the same account")]
    AccountUsedTwice,
    #[error("the mapping names someone who is not on the old laptop")]
    UnknownPerson,
    #[error("no one has decided where {account_id}'s things go")]
    Unmapped { account_id: String },
    #[error("the plan changed after it was approved")]
    StaleApproval,
    #[error("the system's random number source failed")]
    Random,
    #[error("this inventory cannot be read: {0}")]
    BadInventory(&'static str),
    #[error("this laptop's list takes more memory than one move may hold")]
    TooBigForOneMove,
    #[error("the order of the person's own essentials names an item twice or one not in the plan")]
    BadPersonalOrder,
}

/// Writes fixed-size IDs as lowercase hex.
macro_rules! hex_id {
    ($name:ident, $len:expr, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name([u8; $len]);

        impl $name {
            pub fn to_hex(&self) -> String {
                hex::encode(self.0)
            }

            pub fn from_hex(s: &str) -> Result<Self, RecordError> {
                let mut bytes = [0u8; $len];
                hex::decode_to_slice(s, &mut bytes)
                    .map_err(|e| RecordError::BadFormat(e.to_string()))?;
                Ok(Self(bytes))
            }
        }

        impl Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_str(&self.to_hex())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let s = String::deserialize(d)?;
                Self::from_hex(&s).map_err(serde::de::Error::custom)
            }
        }
    };
}

hex_id!(
    LaptopId,
    16,
    "A laptop's random, permanent ID, made once per install."
);
hex_id!(
    ItemId,
    16,
    "An item's permanent ID: the same every time the same laptop is scanned."
);
hex_id!(
    Fingerprint,
    32,
    "A SHA-256 fingerprint of one revision of a record."
);

impl LaptopId {
    pub fn generate() -> Result<Self, RecordError> {
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).map_err(|_| RecordError::Random)?;
        Ok(Self(bytes))
    }
}

impl ItemId {
    /// Derives the ID from what makes an item itself: the laptop, the owner, the place and the
    /// path (by exact bytes). Scanning the same laptop again gives the same IDs.
    pub fn derive(laptop: &LaptopId, owner: &Owner, place: &Place, path: &ItemPath) -> Self {
        let mut h = Sha256::new();
        h.update(b"pctwin/v1/item-id\0");
        h.update(canonical(&(laptop, owner, place, path)));
        let digest: [u8; 32] = h.finalize().into();
        let mut id = [0u8; 16];
        id.copy_from_slice(&digest[..16]);
        Self(id)
    }
}

/// Feeds what is written to a hash.
struct HashWriter<'a>(&'a mut Sha256);

impl std::io::Write for HashWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.update(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// A record's JSON, shown a piece at a time (see [`Record::json_text`]).
pub struct JsonText<'a>(&'a Record);

impl std::fmt::Display for JsonText<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        /// Passes each piece of JSON on as text: JSON is UTF-8, and the writer splits it only
        /// between whole characters.
        struct Text<'a, 'b>(&'a mut std::fmt::Formatter<'b>);
        impl std::io::Write for Text<'_, '_> {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                let text = std::str::from_utf8(buf).map_err(std::io::Error::other)?;
                self.0.write_str(text).map_err(std::io::Error::other)?;
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        serde_json::to_writer(Text(f), self.0).map_err(|_| std::fmt::Error)
    }
}

/// The one serialised form everything is hashed and saved in.
fn canonical<T: Serialize>(value: &T) -> Vec<u8> {
    // Every type here serialises to JSON without failing: no maps with non-text keys, no floats.
    // A silent empty result would give every record the same fingerprint, so fail loudly instead.
    serde_json::to_vec(value).expect("records always serialise")
}

/// A name's exact bytes, kept only when they are not valid text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum RawName {
    /// Bytes from macOS or Linux.
    Unix(Vec<u8>),
    /// UTF-16 units from Windows.
    Windows(Vec<u16>),
}

impl RawName {
    /// The text shown (what `String::from_utf8_lossy` or `from_utf16_lossy` gives), made in one
    /// allocation of exactly its length: no growing, no copy, nothing let go along the way. The
    /// scan inventory counts what a name holds by its length, and blocks let go between the ones
    /// kept leave gaps an allocator may never give back (measured on macOS, 21 October 2026).
    fn lossy(&self) -> String {
        const SHOWN: char = char::REPLACEMENT_CHARACTER;
        match self {
            Self::Unix(b) => {
                let len: usize = b
                    .utf8_chunks()
                    .map(|c| c.valid().len() + if c.invalid().is_empty() { 0 } else { 3 })
                    .sum();
                let mut text = String::with_capacity(len);
                for c in b.utf8_chunks() {
                    text.push_str(c.valid());
                    if !c.invalid().is_empty() {
                        text.push(SHOWN);
                    }
                }
                text
            }
            Self::Windows(w) => {
                let units = || char::decode_utf16(w.iter().copied());
                let len: usize = units().map(|c| c.map_or(3, char::len_utf8)).sum();
                let mut text = String::with_capacity(len);
                text.extend(units().map(|c| c.unwrap_or(SHOWN)));
                text
            }
        }
    }

    /// Whether the exact form is valid text, checked without making the text.
    fn is_valid_text(&self) -> bool {
        match self {
            Self::Unix(b) => std::str::from_utf8(b).is_ok(),
            Self::Windows(w) => char::decode_utf16(w.iter().copied()).all(|c| c.is_ok()),
        }
    }
}

/// One file or folder name, exactly as the disk had it (never normalised).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "NameWire", into = "NameWire")]
pub struct ItemName {
    text: String,
    exact: Option<RawName>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NameWire {
    text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    exact: Option<RawName>,
}

impl ItemName {
    pub fn from_text(text: &str) -> Self {
        Self {
            text: text.to_string(),
            exact: None,
        }
    }

    pub fn from_unix_bytes(bytes: &[u8]) -> Self {
        Self::from_raw(RawName::Unix(bytes.to_vec()))
    }

    pub fn from_windows_wide(wide: &[u16]) -> Self {
        Self::from_raw(RawName::Windows(wide.to_vec()))
    }

    /// A name from exact Windows units, kept as they are given (no copy).
    pub(crate) fn from_windows_units(wide: Vec<u16>) -> Self {
        Self::from_raw(RawName::Windows(wide))
    }

    fn from_raw(raw: RawName) -> Self {
        let text = raw.lossy();
        let exact = (!raw.is_valid_text()).then_some(raw);
        Self { text, exact }
    }

    /// The name to show. For names that are not valid text, unreadable parts show as `�`.
    pub fn display(&self) -> &str {
        &self.text
    }

    /// The exact bytes, present only when the name is not valid text.
    pub fn exact(&self) -> Option<&RawName> {
        self.exact.as_ref()
    }

    /// The name as this system's file APIs take it, exactly as the scan recorded it (a name that
    /// is not valid text keeps its exact bytes on the system it came from).
    pub fn on_disk(&self) -> std::ffi::OsString {
        match &self.exact {
            #[cfg(unix)]
            Some(RawName::Unix(bytes)) => {
                use std::os::unix::ffi::OsStringExt;
                std::ffi::OsString::from_vec(bytes.clone())
            }
            #[cfg(windows)]
            Some(RawName::Windows(units)) => {
                use std::os::windows::ffi::OsStringExt;
                std::ffi::OsString::from_wide(units)
            }
            _ => std::ffi::OsString::from(&self.text),
        }
    }
}

impl TryFrom<NameWire> for ItemName {
    type Error = RecordError;

    fn try_from(wire: NameWire) -> Result<Self, RecordError> {
        if let Some(raw) = &wire.exact {
            // Exact bytes are kept only for names that are not valid text, and the text shown must
            // be what those bytes read as: a record cannot show one name and mean another.
            if raw.is_valid_text() || raw.lossy() != wire.text {
                return Err(RecordError::BadName);
            }
        }
        Ok(Self {
            text: wire.text,
            exact: wire.exact,
        })
    }
}

impl From<ItemName> for NameWire {
    fn from(name: ItemName) -> Self {
        Self {
            text: name.text,
            exact: name.exact,
        }
    }
}

/// Whether `t` cannot be one part of a path: empty, `.`, `..`, or holding `/` or a NUL.
pub(crate) fn breaks_part_rules(t: &str) -> bool {
    t.is_empty() || t == "." || t == ".." || t.contains(['/', '\0'])
}

/// Where an item sits inside its [`Place`], one name per folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Vec<ItemName>", into = "Vec<ItemName>")]
pub struct ItemPath(Vec<ItemName>);

impl ItemPath {
    /// Refuses an empty path and any part that is empty, `.`, `..`, or holds `/` or a NUL.
    pub fn new(parts: Vec<ItemName>) -> Result<Self, RecordError> {
        if parts.is_empty() || parts.iter().any(|n| breaks_part_rules(n.display())) {
            return Err(RecordError::BadName);
        }
        Ok(Self(parts))
    }

    pub fn parts(&self) -> &[ItemName] {
        &self.0
    }
}

impl TryFrom<Vec<ItemName>> for ItemPath {
    type Error = RecordError;

    fn try_from(parts: Vec<ItemName>) -> Result<Self, RecordError> {
        Self::new(parts)
    }
}

impl From<ItemPath> for Vec<ItemName> {
    fn from(path: ItemPath) -> Self {
        path.0
    }
}

/// What a folder is for, whatever it is called and wherever it really is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FolderRole {
    Home,
    Desktop,
    Documents,
    Downloads,
    Pictures,
    Music,
    Videos,
    Public,
    AppData,
    Other,
}

/// A cloud storage service whose folder is on the laptop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CloudProvider {
    OneDrive,
    ICloud,
    Dropbox,
    GoogleDrive,
    Other,
}

/// Where a folder really is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum Storage {
    /// The drive the system runs from.
    SystemDrive,
    /// Another drive, by the ID the scan gave it.
    OtherDrive { drive: String },
    /// A cloud storage folder (OneDrive, iCloud and others).
    Cloud { provider: CloudProvider },
}

/// Where an item really lives: what the folder is for, and where that folder is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Place {
    pub role: FolderRole,
    pub storage: Storage,
}

/// Whose an item is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum Owner {
    /// One person, by the system's own account ID (never by name).
    Person { account_id: String },
    /// Belongs to everyone on the laptop (the Public folder, apps installed for all users).
    Shared,
    /// Part of the system.
    System,
}

/// Whether an item can move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Portability {
    /// Can be copied.
    Portable,
    /// Can be rebuilt on the new laptop, for example by installing the app.
    Rebuildable,
    /// Tied to the old laptop (authenticators, passkeys, Windows Hello).
    DeviceBound,
    /// A licence with an install limit; the old laptop may need deactivating first.
    LimitedActivations,
}

/// Who manages an item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ManagedBy {
    Personal,
    /// An employer or school ("OneDrive - Company", a managed laptop).
    Organisation,
}

/// What an item is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ItemKind {
    File,
    Folder,
    App,
    Setting,
    /// A folder that moves as one whole (a Mac package such as a Photos Library or a Pages
    /// document, or a project's history such as `.git`): its files are listed under it and sent
    /// like any other, and on the new laptop they land inside one folder of its own, never merged
    /// into a folder already there. It carries no bytes itself. Move protocol 4 and later.
    Package,
}

/// Why an item is left out of the move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LeftOutReason {
    ChosenByUser,
    WorkManaged,
    DeviceBound,
    Unreadable,
    CloudOnly,
    System,
    /// A folder with more files than one move's list can name: counted as one item and not
    /// moved (it comes with the record that lists any number of items). Move protocol 4 and
    /// later; an older new laptop is asked to update.
    TooManyFiles,
    /// It can be made again on the new laptop (downloaded packages, build output such as
    /// `node_modules`), so it is counted as one item and not copied (the rules file's
    /// "rebuildable"). Move protocol 4 and later.
    Rebuildable,
}

/// Whether an item is part of the move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum Inclusion {
    Included,
    LeftOut { reason: LeftOutReason },
}

/// One thing found on the old laptop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Item {
    pub id: ItemId,
    pub kind: ItemKind,
    pub owner: Owner,
    pub place: Place,
    pub path: ItemPath,
    pub size_bytes: u64,
    /// When it was last changed at the scan, in nanoseconds since 1970, when the system gives it.
    /// With its size, what the move compares to tell a file changed since the scan.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified_ns: Option<i64>,
    pub portability: Portability,
    pub managed_by: ManagedBy,
    /// How much a rebuild downloads, when known.
    pub download_bytes: Option<u64>,
    pub inclusion: Inclusion,
}

/// Where one person's things go on the new laptop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum PersonTarget {
    /// An account that already exists on the new laptop, by its account ID.
    ExistingAccount { account_id: String },
    /// A new account to create (with the administrator prompt).
    NewAccount { name: String },
    /// This person's things are not moved.
    Skip,
}

/// Where shared things go on the new laptop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum SharedTarget {
    SharedFolder,
    Account { account_id: String },
    Skip,
}

/// Where everything goes, decided in "Who's moving?". Keys are the old laptop's account IDs; a
/// matching name is only ever a suggestion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mapping {
    pub people: BTreeMap<String, PersonTarget>,
    pub shared: SharedTarget,
}

/// One revision of the move record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    format: u32,
    pub source_laptop: LaptopId,
    pub revision: u64,
    /// The fingerprint of the revision before this one.
    pub previous: Option<Fingerprint>,
    pub items: Vec<Item>,
    pub mapping: Mapping,
    /// The person's own essentials, first first: items of this record they used most recently
    /// or keep pinned, which move before the rest (Product Spec "personal essentials", decided
    /// 24 October 2026). Only this order is kept and crosses to their own new laptop, over the
    /// sealed pairing link: never when or how anything was used, and never to anyone else. It is
    /// part of the plan the person approves (it counts in the fingerprint). At most
    /// [`MAX_PERSONAL`]. Written only when there is one, so a record without it is written and
    /// fingerprinted exactly as before.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub personal: Vec<ItemId>,
    /// On the new laptop, a record built from the old laptop's paged list (scan inventory version
    /// 2 and later): the fingerprint of that list, taken over its bytes exactly as they crossed
    /// (Scan record version 2, point 6). It counts in this record's own fingerprint, so the plan
    /// the person approves names the very list that arrived, never a re-encoding of it. Written
    /// only when there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inventory: Option<Fingerprint>,
}

/// Most items in the personal order (the most recently used are kept).
pub const MAX_PERSONAL: usize = 10_000;

/// The person's approval of one revision of the plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Approval {
    pub revision: u64,
    pub fingerprint: Fingerprint,
}

impl Record {
    /// The first revision of an empty record for `source_laptop`. Shared things go to the shared
    /// folder unless the person chooses otherwise.
    pub fn new(source_laptop: LaptopId) -> Self {
        Self {
            format: FORMAT,
            source_laptop,
            revision: 1,
            previous: None,
            items: Vec::new(),
            mapping: Mapping {
                people: BTreeMap::new(),
                shared: SharedTarget::SharedFolder,
            },
            personal: Vec::new(),
            inventory: None,
        }
    }

    pub fn format(&self) -> u32 {
        self.format
    }

    /// The record as JSON, whole, in memory. Only for a record known to be small (a test, or
    /// move protocol 2, which caps it): a record received in pages can be far larger than any
    /// one text should be, so it is written with [`write_json`](Self::write_json) and
    /// fingerprinted by [`fingerprint`](Self::fingerprint), both a piece at a time.
    pub fn to_json(&self) -> Vec<u8> {
        canonical(self)
    }

    /// Writes the record as JSON to `out` a piece at a time (the same bytes as
    /// [`to_json`](Self::to_json)), never holding the whole text.
    pub fn write_json(&self, out: impl std::io::Write) -> std::io::Result<()> {
        serde_json::to_writer(out, self).map_err(std::io::Error::other)
    }

    /// The record's JSON as something that can be written into a text (a saved scan holds it as
    /// one), a piece at a time.
    pub fn json_text(&self) -> JsonText<'_> {
        JsonText(self)
    }

    /// Loads a saved record. The format is checked first, so a newer record is refused safely even
    /// if its shape has changed; then the record is checked like a new one.
    pub fn from_json(bytes: &[u8]) -> Result<Self, RecordError> {
        RECORD_FORMAT.check_json(bytes).map_err(|e| match e {
            FormatError::Newer { found, .. } => RecordError::NewerFormat { found },
            other => RecordError::BadFormat(other.to_string()),
        })?;
        let record: Self =
            serde_json::from_slice(bytes).map_err(|e| RecordError::BadFormat(e.to_string()))?;
        record.validate()?;
        Ok(record)
    }

    /// The fingerprint of this exact revision: every field counts. Taken over the same bytes as
    /// [`to_json`](Self::to_json), fed to the hash as they are made, so the whole text is never
    /// held.
    pub fn fingerprint(&self) -> Fingerprint {
        let mut h = Sha256::new();
        h.update(b"pctwin/v1/record\0");
        // Every type here serialises to JSON without failing (see `canonical`), and the hash
        // never refuses bytes.
        serde_json::to_writer(HashWriter(&mut h), self).expect("records always serialise");
        Fingerprint(h.finalize().into())
    }

    /// The next revision, linked to this one, as a copy. Change it, then approve it again. A
    /// received record can be large: when this one is not needed afterwards, use
    /// [`into_next_revision`](Self::into_next_revision), which makes no copy.
    pub fn next_revision(&self) -> Self {
        self.clone().into_next_revision()
    }

    /// The next revision, linked to this one, made from this one in place (no copy).
    pub fn into_next_revision(mut self) -> Self {
        self.previous = Some(self.fingerprint());
        self.revision += 1;
        self
    }

    /// The rules every record keeps.
    pub fn validate(&self) -> Result<(), RecordError> {
        let mut ids = HashSet::new();
        let mut people = HashSet::new();
        for item in &self.items {
            if !ids.insert(item.id) {
                return Err(RecordError::DuplicateId);
            }
            if item.portability == Portability::DeviceBound && item.inclusion == Inclusion::Included
            {
                return Err(RecordError::DeviceBoundIncluded);
            }
            if let Owner::Person { account_id } = &item.owner {
                people.insert(account_id.as_str());
            }
        }
        // The personal order names only this record's own items, each once.
        if self.personal.len() > MAX_PERSONAL {
            return Err(RecordError::BadPersonalOrder);
        }
        let mut ordered = HashSet::new();
        if self
            .personal
            .iter()
            .any(|id| !ids.contains(id) || !ordered.insert(*id))
        {
            return Err(RecordError::BadPersonalOrder);
        }
        let mut existing = HashSet::new();
        let mut created = HashSet::new();
        for (person, target) in &self.mapping.people {
            if !people.contains(person.as_str()) {
                return Err(RecordError::UnknownPerson);
            }
            let fresh = match target {
                PersonTarget::ExistingAccount { account_id } => {
                    existing.insert(account_id.as_str())
                }
                PersonTarget::NewAccount { name } => created.insert(name.to_lowercase()),
                PersonTarget::Skip => true,
            };
            if !fresh {
                return Err(RecordError::AccountUsedTwice);
            }
        }
        Ok(())
    }

    /// Approves this revision. Everyone with something included must have a place to go.
    pub fn approve(&self) -> Result<Approval, RecordError> {
        self.validate()?;
        for item in &self.items {
            if let (Owner::Person { account_id }, Inclusion::Included) =
                (&item.owner, item.inclusion)
                && !self.mapping.people.contains_key(account_id)
            {
                return Err(RecordError::Unmapped {
                    account_id: account_id.clone(),
                });
            }
        }
        Ok(Approval {
            revision: self.revision,
            fingerprint: self.fingerprint(),
        })
    }

    /// Checks `approval` was given for exactly this revision, unchanged, and that this revision
    /// still passes every rule [`approve`](Self::approve) applies. An approval is only a pair of
    /// numbers anyone can write, so it never stands in for those rules: a record that could not be
    /// approved is refused here too, whatever approval comes with it.
    pub fn check(&self, approval: &Approval) -> Result<(), RecordError> {
        if self.approve()? == *approval {
            Ok(())
        } else {
            Err(RecordError::StaleApproval)
        }
    }
}

#[cfg(test)]
mod name_room {
    use super::*;

    /// A record with names of every kind, an owner and a mapping.
    fn awkward() -> Record {
        let laptop = LaptopId([3; 16]);
        let mut r = Record::new(laptop);
        for (i, name) in [
            ItemName::from_text("Résumé \"final\" \\ notes\n.txt"),
            ItemName::from_unix_bytes(&[b'a', 0xFF, b'b']),
            ItemName::from_windows_wide(&[0xD800, 0x61, 0x4E2D]),
        ]
        .into_iter()
        .enumerate()
        {
            let owner = Owner::Person {
                account_id: format!("S-1-5-21-{i}"),
            };
            let place = Place {
                role: FolderRole::Documents,
                storage: Storage::SystemDrive,
            };
            let path = ItemPath::new(vec![ItemName::from_text("文件"), name]).unwrap();
            r.items.push(Item {
                id: ItemId::derive(&laptop, &owner, &place, &path),
                kind: ItemKind::File,
                owner,
                place,
                path,
                size_bytes: 7,
                modified_ns: None,
                portability: Portability::Portable,
                managed_by: ManagedBy::Personal,
                download_bytes: Some(3),
                inclusion: Inclusion::Included,
            });
        }
        r.mapping.people.insert(
            "S-1-5-21-0".into(),
            PersonTarget::NewAccount { name: "Ada".into() },
        );
        r
    }

    /// The fingerprint, the JSON written a piece at a time, and the JSON shown as text are all
    /// the very bytes of the whole JSON, so nothing changes for records already saved or approved.
    #[test]
    fn written_a_piece_at_a_time_it_is_the_same_bytes_and_the_same_fingerprint() {
        let r = awkward();
        let whole = r.to_json();
        let mut h = Sha256::new();
        h.update(b"pctwin/v1/record\0");
        h.update(&whole);
        assert_eq!(r.fingerprint(), Fingerprint(h.finalize().into()));
        let mut written = Vec::new();
        r.write_json(&mut written).unwrap();
        assert_eq!(written, whole);
        assert_eq!(r.json_text().to_string().into_bytes(), whole);
        let next = r.clone().into_next_revision();
        assert_eq!(next, r.next_revision());
        assert_eq!(next.previous, Some(r.fingerprint()));
        assert_eq!(next.revision, r.revision + 1);
    }

    /// The text of a name that is not valid text holds exactly its length, as the scan
    /// inventory counts it, and reads as the standard lossy conversion does.
    #[test]
    fn the_text_of_a_name_kept_exactly_holds_only_its_length() {
        let mut bytes = vec![0xFF];
        bytes.extend(std::iter::repeat_n(b'a', 1364));
        let mut wide = vec![0xD800];
        wide.extend(std::iter::repeat_n(0x61, 3999));
        for name in [
            ItemName::from_unix_bytes(&bytes),
            ItemName::from_windows_wide(&wide),
        ] {
            assert!(name.exact().is_some());
            assert_eq!(name.text.capacity(), name.text.len());
        }
        let odd: [&[u8]; 4] = [b"a\xFF\xFEb", b"\xE2\x82", b"ok\xF0\x9F\x98", b"\xC3"];
        for b in odd {
            let name = ItemName::from_unix_bytes(b);
            assert_eq!(name.display(), String::from_utf8_lossy(b));
            assert_eq!(name.text.capacity(), name.text.len());
        }
        let units: [&[u16]; 3] = [&[0xD800, 0x61], &[0x61, 0xDC00, 0xD83D, 0xDE00], &[0xDBFF]];
        for w in units {
            let name = ItemName::from_windows_wide(w);
            assert_eq!(name.display(), String::from_utf16_lossy(w));
            assert_eq!(name.text.capacity(), name.text.len());
            assert_eq!(name.exact().is_some(), String::from_utf16(w).is_err());
        }
    }
}
