//! How a received file's contents were checked before it got its name: read back from the drive,
//! or through this laptop's memory and why (see `pctwin_gate::read_back`). Kept here, in the
//! change journal, so the report after a restart still says it.

pctwin_types::listed_enum! {
    /// Where the bytes of a read-back came from.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum CheckedFrom {
        /// Read from the drive itself, past this laptop's cache.
        Drive,
        /// Read through this laptop's cache: the file was still checked, but its bytes may have come
        /// from this laptop's memory rather than the drive. "More details" says so.
        Memory(WhyMemory),
    }
}

impl CheckedFrom {
    /// Every value, drive first, then memory for each reason in [`WhyMemory::ALL`]'s order (its
    /// test checks it against `VARIANTS`).
    pub const ALL: [CheckedFrom; 6] = [
        Self::Drive,
        Self::Memory(WhyMemory::CannotSkipCache),
        Self::Memory(WhyMemory::NotShown),
        Self::Memory(WhyMemory::NoWay),
        Self::Memory(WhyMemory::StoredSpecially),
        Self::Memory(WhyMemory::TooSmall),
    ];
}

/// The screens' shape (app/src/move.ts `Evidence`): `{ "from": "drive" }`, or
/// `{ "from": "memory", "why": "<reason>" }` with the reason in kebab-case.
impl serde::Serialize for CheckedFrom {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        match self {
            Self::Drive => {
                let mut s = serializer.serialize_struct("CheckedFrom", 1)?;
                s.serialize_field("from", "drive")?;
                s.end()
            }
            Self::Memory(why) => {
                let mut s = serializer.serialize_struct("CheckedFrom", 2)?;
                s.serialize_field("from", "memory")?;
                s.serialize_field("why", why)?;
                s.end()
            }
        }
    }
}

pctwin_types::listed_enum! {
    /// Why a check could not read past this laptop's cache. Sent to the screens by its kebab-case
    /// name, which is also the key of its words there (`why-memory-<name>`).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum WhyMemory {
        /// The drive refused to be read past the cache (some network drives and USB sticks).
        CannotSkipCache,
        /// The drive accepted a read past the cache but did not show the read reached the drive (it
        /// allowed a read that only the cache could answer), so it is not counted as one.
        NotShown,
        /// This system has no way to read this drive past its cache.
        NoWay,
        /// The file is stored in a way the system reads through its memory (compressed, encrypted,
        /// kept inline or journalled), whatever the drive allows.
        StoredSpecially,
        /// The file is so small the drive may keep it inside its own records, which the system reads
        /// through its memory.
        TooSmall,
    }
}

impl WhyMemory {
    /// Every reason, in the screens' order (app/src/move.ts `WHY_MEMORY`); its test checks it
    /// against `VARIANTS`.
    pub const ALL: [WhyMemory; 5] = [
        Self::CannotSkipCache,
        Self::NotShown,
        Self::NoWay,
        Self::StoredSpecially,
        Self::TooSmall,
    ];
}

impl std::fmt::Display for CheckedFrom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Drive => f.write_str("checked by reading it back from the drive"),
            Self::Memory(why) => write!(
                f,
                "checked from this laptop's memory, not the drive, because {why}"
            ),
        }
    }
}

impl std::fmt::Display for WhyMemory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::CannotSkipCache => "this drive cannot be read without the system's memory cache",
            Self::NotShown => "this drive could not show that it was read directly",
            Self::NoWay => "this system cannot read this drive directly",
            Self::StoredSpecially => {
                "this file is stored compressed or protected, which the system reads through memory"
            }
            Self::TooSmall => "this file is too small to be sure the drive itself was read",
        })
    }
}

/// Reads back what [`CheckedFrom`]'s `Serialize` wrote: `{ "from": "drive" }`, or
/// `{ "from": "memory", "why": "<reason>" }`.
impl<'de> serde::Deserialize<'de> for CheckedFrom {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Shape {
            from: String,
            why: Option<WhyMemory>,
        }
        let s = Shape::deserialize(deserializer)?;
        match (s.from.as_str(), s.why) {
            ("drive", None) => Ok(Self::Drive),
            ("memory", Some(why)) => Ok(Self::Memory(why)),
            _ => Err(serde::de::Error::custom("not a way a file was checked")),
        }
    }
}
