//! The shared move record (Product Spec "How PCTwin understands two real laptops"; Task List 0.6).
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
//!   (Engineering Plan 9.11), and a loaded record is checked like a new one.

use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The record format this app writes and reads.
pub const FORMAT: u32 = 1;

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
    fn lossy(&self) -> String {
        match self {
            Self::Unix(b) => String::from_utf8_lossy(b).into_owned(),
            Self::Windows(w) => String::from_utf16_lossy(w),
        }
    }

    fn is_valid_text(&self) -> bool {
        match self {
            Self::Unix(b) => std::str::from_utf8(b).is_ok(),
            Self::Windows(w) => String::from_utf16(w).is_ok(),
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

/// Where an item sits inside its [`Place`], one name per folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Vec<ItemName>", into = "Vec<ItemName>")]
pub struct ItemPath(Vec<ItemName>);

impl ItemPath {
    /// Refuses an empty path and any part that is empty, `.`, `..`, or holds `/` or a NUL.
    pub fn new(parts: Vec<ItemName>) -> Result<Self, RecordError> {
        let bad = |n: &ItemName| {
            let t = n.display();
            t.is_empty() || t == "." || t == ".." || t.contains(['/', '\0'])
        };
        if parts.is_empty() || parts.iter().any(bad) {
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
}

/// The person's approval of one revision of the plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Approval {
    pub revision: u64,
    pub fingerprint: Fingerprint,
}

#[derive(Deserialize)]
struct FormatOnly {
    format: u32,
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
        }
    }

    pub fn format(&self) -> u32 {
        self.format
    }

    pub fn to_json(&self) -> Vec<u8> {
        canonical(self)
    }

    /// Loads a saved record. The format is checked first, so a newer record is refused safely even
    /// if its shape has changed; then the record is checked like a new one.
    pub fn from_json(bytes: &[u8]) -> Result<Self, RecordError> {
        let FormatOnly { format } =
            serde_json::from_slice(bytes).map_err(|e| RecordError::BadFormat(e.to_string()))?;
        if format > FORMAT {
            return Err(RecordError::NewerFormat { found: format });
        }
        if format != FORMAT {
            return Err(RecordError::BadFormat(format!("unknown format {format}")));
        }
        let record: Self =
            serde_json::from_slice(bytes).map_err(|e| RecordError::BadFormat(e.to_string()))?;
        record.validate()?;
        Ok(record)
    }

    /// The fingerprint of this exact revision: every field counts.
    pub fn fingerprint(&self) -> Fingerprint {
        let mut h = Sha256::new();
        h.update(b"pctwin/v1/record\0");
        h.update(self.to_json());
        Fingerprint(h.finalize().into())
    }

    /// The next revision, linked to this one. Change it, then approve it again.
    pub fn next_revision(&self) -> Self {
        let mut next = self.clone();
        next.revision = self.revision + 1;
        next.previous = Some(self.fingerprint());
        next
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

    /// Checks `approval` was given for exactly this revision, unchanged.
    pub fn check(&self, approval: &Approval) -> Result<(), RecordError> {
        if approval.revision == self.revision && approval.fingerprint == self.fingerprint() {
            Ok(())
        } else {
            Err(RecordError::StaleApproval)
        }
    }
}
