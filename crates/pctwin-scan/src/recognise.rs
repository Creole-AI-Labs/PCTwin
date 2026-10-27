//! What each place is (`rules/what.json`, the `place-kinds` catalogue): markers and folder
//! shapes that name a place for the screens (camera photos, a phone backup, an old backup of the
//! laptop), run over the mover's own record after the sweep, so another person's things are never
//! looked at. A label never changes what is included, whose it is or where it lands; it names,
//! orders and weighs.
//!
//! Kinds of place are rows too (Phase 2 adoption, changes 7 and 8): a `kind` row gives a kind's
//! ID, its name and its traits (a closed set the code acts on: `old-backup`, `fetch-again`,
//! `fetch-again-if-downloaded`); a `marker` row says how a place of that kind looks:
//!
//! - `system`: `windows`, `mac`, `linux` or `any`;
//! - `shape`: `folder` (a folder with this name), `folder-suffix` (a folder whose name ends with
//!   this, a package such as `.photoslibrary`), `holds` (a folder that directly holds a file or
//!   folder with this name) or `file-suffix` (a file whose name ends with this);
//! - `what`: a kind's ID.
//!
//! Names are compared ignoring case; the first marker row that matches names the place. A row
//! this version cannot use (an unknown kind, shape or system, a newer field) is skipped and
//! reported ([`Recogniser::report`]), never a reason to stop: a newer table never stops an older
//! app. The shipped table is compiled into the app, so it is covered by the app's own signature;
//! a test pins its fingerprint, so a change to it is a change to the app. Folders at the top of
//! a place that no row names are listed as found, not named ([`Named::found_not_named`]).

use std::collections::HashMap;

use pctwin_catalogue::{Catalogue, Report, Row};
use pctwin_record::{Item, ItemId, ItemKind, ItemPath, Record};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::rules::System;

/// What a place is: a kind's ID from the table (1 to [`What::MAX_ID`] bytes of `a-z`, `0-9` and
/// `-`). The kinds the shipped table holds are named here too.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct What {
    len: u8,
    id: [u8; What::MAX_ID],
}

#[allow(non_upper_case_globals)]
impl What {
    pub const MAX_ID: usize = 31;
    /// Photos straight from a camera or phone.
    pub const CameraPhotos: Self = Self::fixed("camera-photos");
    pub const PhotosLibrary: Self = Self::fixed("photos-library");
    pub const LightroomCatalog: Self = Self::fixed("lightroom-catalog");
    pub const MusicLibrary: Self = Self::fixed("music-library");
    /// An iPhone or iPad backup.
    pub const PhoneBackup: Self = Self::fixed("phone-backup");
    pub const TimeMachine: Self = Self::fixed("time-machine");
    pub const FileHistory: Self = Self::fixed("file-history");
    pub const WindowsImageBackup: Self = Self::fixed("windows-image-backup");
    /// The older copy of Windows an upgrade leaves.
    pub const OldWindows: Self = Self::fixed("old-windows");
    pub const VirtualMachineDisk: Self = Self::fixed("virtual-machine-disk");
    /// A Linux system kept inside Windows (WSL).
    pub const LinuxOnWindows: Self = Self::fixed("linux-on-windows");
    pub const Containers: Self = Self::fixed("containers");
    pub const GameSaves: Self = Self::fixed("game-saves");
    pub const InstalledGames: Self = Self::fixed("installed-games");
    pub const Screenshots: Self = Self::fixed("screenshots");
    /// A backup program's image of a whole drive.
    pub const DiskImageBackup: Self = Self::fixed("disk-image-backup");
    /// An installer or disc image.
    pub const DiscImage: Self = Self::fixed("disc-image");
    /// Mail kept on the laptop, the person's own.
    pub const EmailArchive: Self = Self::fixed("email-archive");
    /// Outlook's offline copy of mail kept on a server: it comes back by itself.
    pub const EmailCache: Self = Self::fixed("email-cache");
    pub const PasswordVault: Self = Self::fixed("password-vault");
    pub const MoneyRecords: Self = Self::fixed("money-records");
    pub const CodeProject: Self = Self::fixed("code-project");
    pub const NotesVault: Self = Self::fixed("notes-vault");
    pub const VideoProject: Self = Self::fixed("video-project");
    pub const MusicProject: Self = Self::fixed("music-project");
    pub const DesignFile: Self = Self::fixed("design-file");
    pub const EbookLibrary: Self = Self::fixed("ebook-library");

    const fn fixed(id: &str) -> Self {
        let b = id.as_bytes();
        assert!(Self::ok(b), "a fixed kind ID is valid");
        let mut out = [0u8; Self::MAX_ID];
        let mut i = 0;
        while i < b.len() {
            out[i] = b[i];
            i += 1;
        }
        Self {
            len: b.len() as u8,
            id: out,
        }
    }

    const fn ok(b: &[u8]) -> bool {
        if b.is_empty() || b.len() > Self::MAX_ID || b[0] == b'-' {
            return false;
        }
        let mut i = 0;
        while i < b.len() {
            if !(b[i].is_ascii_lowercase() || b[i].is_ascii_digit() || b[i] == b'-') {
                return false;
            }
            i += 1;
        }
        true
    }

    /// A kind by its ID, checked.
    pub fn new(id: &str) -> Option<Self> {
        Self::ok(id.as_bytes()).then(|| Self::fixed(id))
    }

    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.id[..usize::from(self.len)]).unwrap_or("")
    }

    /// An old backup of a laptop: worth keeping, but likely the same things again (the kind's
    /// `old-backup` trait in the shipped table).
    pub fn is_old_backup(self) -> bool {
        Recogniser::shipped().has(self, Trait::OldBackup)
    }
}

impl std::fmt::Debug for What {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "What({})", self.as_str())
    }
}

impl Serialize for What {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for What {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Self::new(&s).ok_or_else(|| serde::de::Error::custom("not a kind of place"))
    }
}

/// What the code does with a kind (a closed set: code per trait, never per kind).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Trait {
    /// An old backup of a laptop.
    OldBackup,
    /// Can be fetched again (installed games; a server's mail).
    FetchAgain,
    /// Can be fetched again when its folder's files were all downloaded (an installer).
    FetchAgainIfDownloaded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum RowSystem {
    Windows,
    Mac,
    Linux,
    Any,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Shape {
    Folder,
    FolderSuffix,
    Holds,
    FileSuffix,
}

/// A kind of place.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KindRow {
    pub id: What,
    /// Its name, to show.
    pub name: String,
    #[serde(default)]
    pub traits: Vec<Trait>,
}

/// How a place of a kind looks.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarkerRow {
    system: RowSystem,
    shape: Shape,
    marker: String,
    what: What,
}

/// One row of the `place-kinds` catalogue.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum WhatRow {
    Kind(KindRow),
    Marker(MarkerRow),
}

impl Row for WhatRow {
    const CATALOGUE: &'static str = "place-kinds";
    const FIELDS: &'static [&'static str] = &["kind", "marker"];
    fn check(&self) -> Result<(), String> {
        match self {
            WhatRow::Kind(k) => {
                if k.name.trim().is_empty()
                    || k.name.len() > 64
                    || k.name.chars().any(char::is_control)
                {
                    return Err("a kind's name is 1 to 64 plain characters".into());
                }
            }
            WhatRow::Marker(m) => {
                if m.marker.trim().is_empty() || m.marker.len() > 128 {
                    return Err("an empty marker".into());
                }
            }
        }
        Ok(())
    }
}

/// One marker this build uses.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Marker {
    shape: Shape,
    /// Lower case: names are compared ignoring case.
    marker: String,
    what: What,
    /// Its place among the table's rows that loaded, from 1 (the file's own row when nothing
    /// was skipped, as in the shipped table).
    line: usize,
}

/// The table, read once, for one system.
#[derive(Debug, Clone)]
pub struct Recogniser {
    kinds: Vec<KindRow>,
    markers: Vec<Marker>,
    report: Report,
    /// Marker rows naming a kind no kind row gives (row number, the kind): skipped.
    unknown_kinds: Vec<(usize, What)>,
}

/// A place the table named: the item, what it is, and the table's row that named it (its place
/// among the rows that loaded, from 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recognised {
    pub item: ItemId,
    pub what: What,
    pub line: usize,
}

/// What the table made of a record: every place it named, and every folder at the top of a
/// place that it did not (found, not named; listed so the table can grow).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Named {
    pub named: Vec<Recognised>,
    pub found_not_named: Vec<ItemId>,
}

const TABLE: &str = include_str!("../rules/what.json");

impl Recogniser {
    /// The table shipped with PCTwin, for the system this build runs on. Never panics: a table
    /// that cannot be read names nothing, and says why in [`Recogniser::report`].
    pub fn shipped() -> &'static Self {
        static SHIPPED: std::sync::OnceLock<Recogniser> = std::sync::OnceLock::new();
        SHIPPED.get_or_init(|| Self::from_catalogue(&Catalogue::shipped(TABLE), System::this()))
    }

    /// Reads a table's text for `system` (rows it cannot use are skipped and reported).
    pub fn parse(text: &str, system: System) -> Self {
        Self::from_catalogue(&Catalogue::shipped(text), system)
    }

    /// The table in a loaded catalogue (shipped or delivered), for `system`.
    pub fn from_catalogue(table: &Catalogue<WhatRow>, system: System) -> Self {
        let kinds: Vec<KindRow> = table
            .rows()
            .iter()
            .filter_map(|r| match r {
                WhatRow::Kind(k) => Some(k.clone()),
                WhatRow::Marker(_) => None,
            })
            .collect();
        let mut markers = Vec::new();
        let mut unknown_kinds = Vec::new();
        for (i, r) in table.rows().iter().enumerate() {
            let WhatRow::Marker(m) = r else { continue };
            if !kinds.iter().any(|k| k.id == m.what) {
                unknown_kinds.push((i + 1, m.what));
                continue;
            }
            let for_us = match m.system {
                RowSystem::Any => true,
                RowSystem::Windows => system == System::Windows,
                RowSystem::Mac => system == System::Mac,
                RowSystem::Linux => system == System::Linux,
            };
            if for_us {
                markers.push(Marker {
                    shape: m.shape,
                    marker: m.marker.to_lowercase(),
                    what: m.what,
                    line: i + 1,
                });
            }
        }
        Self {
            kinds,
            markers,
            report: table.report().clone(),
            unknown_kinds,
        }
    }

    /// What could not be used: rows skipped by the catalogue, and markers of unknown kinds.
    pub fn report(&self) -> (&Report, &[(usize, What)]) {
        (&self.report, &self.unknown_kinds)
    }

    /// A kind's row.
    pub fn kind(&self, what: What) -> Option<&KindRow> {
        self.kinds.iter().find(|k| k.id == what)
    }

    /// Whether a kind has a trait.
    pub fn has(&self, what: What, t: Trait) -> bool {
        self.kind(what).is_some_and(|k| k.traits.contains(&t))
    }

    /// What a folder named `name`, directly holding `children`, is (the first row that matches).
    pub fn folder<'a>(
        &self,
        name: &str,
        children: impl IntoIterator<Item = &'a str> + Clone,
    ) -> Option<(What, usize)> {
        let name = name.to_lowercase();
        self.markers.iter().find_map(|r| {
            let hit = match r.shape {
                Shape::Folder => name == r.marker,
                Shape::FolderSuffix => name.ends_with(&r.marker),
                Shape::Holds => children
                    .clone()
                    .into_iter()
                    .any(|c| c.to_lowercase() == r.marker),
                Shape::FileSuffix => false,
            };
            hit.then_some((r.what, r.line))
        })
    }

    /// What a file named `name` is.
    pub fn file(&self, name: &str) -> Option<(What, usize)> {
        let name = name.to_lowercase();
        self.markers
            .iter()
            .find(|r| r.shape == Shape::FileSuffix && name.ends_with(&r.marker))
            .map(|r| (r.what, r.line))
    }

    /// Every folder and file of `record` the table names, and every folder at the top of a place
    /// it does not. Only the record is read (the mover's own and shared things), never the drive.
    /// A folder inside one already named is not named again (the photos inside a phone backup are
    /// the backup's).
    pub fn record(&self, record: &Record) -> Named {
        // Each folder's children's names, by the folder's own ID.
        let mut children: HashMap<ItemId, Vec<&str>> = HashMap::new();
        for i in &record.items {
            let parts = i.path.parts();
            let Some((name, parent)) = parts.split_last() else {
                continue;
            };
            let Ok(parent) = ItemPath::new(parent.to_vec()) else {
                continue;
            };
            let folder = ItemId::derive(&record.source_laptop, &i.owner, &i.place, &parent);
            children.entry(folder).or_default().push(name.display());
        }
        let mut found: Vec<(&Item, Recognised)> = Vec::new();
        for i in &record.items {
            let Some(name) = i.path.parts().last() else {
                continue;
            };
            let hit = match i.kind {
                ItemKind::File => self.file(name.display()),
                _ => {
                    let held = children.get(&i.id).map(Vec::as_slice).unwrap_or_default();
                    self.folder(name.display(), held.iter().copied())
                }
            };
            if let Some((what, line)) = hit {
                found.push((
                    i,
                    Recognised {
                        item: i.id,
                        what,
                        line,
                    },
                ));
            }
        }
        // Outermost first, so a place inside one already named is left as part of it.
        found.sort_by_key(|(i, _)| i.path.parts().len());
        let mut kept: Vec<(&Item, Recognised)> = Vec::new();
        for (i, r) in found {
            let parts = i.path.parts();
            let inside = kept.iter().any(|(outer, _)| {
                let o = outer.path.parts();
                outer.owner == i.owner
                    && outer.place == i.place
                    && parts.len() > o.len()
                    && parts.starts_with(o)
            });
            if !inside {
                kept.push((i, r));
            }
        }
        let found_not_named = record
            .items
            .iter()
            .filter(|i| i.kind == ItemKind::Folder && i.path.parts().len() == 1)
            .filter(|i| !kept.iter().any(|(k, _)| k.id == i.id))
            .map(|i| i.id)
            .collect();
        Named {
            named: kept.into_iter().map(|(_, r)| r).collect(),
            found_not_named,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(system: System) -> Recogniser {
        Recogniser::parse(TABLE, system)
    }

    #[test]
    fn the_shipped_table_reads_whole_on_every_system() {
        for system in [System::Windows, System::Mac, System::Linux] {
            let r = table(system);
            assert!(r.markers.len() >= 18, "{system:?}");
            assert!(r.report.is_clean(), "{:?}", r.report);
            assert!(r.unknown_kinds.is_empty());
            assert_eq!(r.kinds.len(), 27);
        }
        assert!(Recogniser::shipped().report().0.is_clean());
    }

    /// The table is part of the app: a change to it is a change to the app, made on purpose.
    #[test]
    fn the_shipped_table_is_the_one_reviewed() {
        let fingerprint = blake3::hash(TABLE.replace("\r\n", "\n").as_bytes());
        assert_eq!(
            fingerprint.to_hex().as_str(),
            include_str!("../rules/what.json.blake3").trim(),
            "rules/what.json changed: review it, then update rules/what.json.blake3"
        );
    }

    #[test]
    fn folders_and_files_are_named_by_their_markers() {
        let r = table(System::Windows);
        assert_eq!(r.folder("DCIM", []).map(|w| w.0), Some(What::CameraPhotos));
        assert_eq!(r.folder("dcim", []).map(|w| w.0), Some(What::CameraPhotos));
        assert_eq!(
            r.folder("00008030-001A", ["Info.plist", "Manifest.db"])
                .map(|w| w.0),
            Some(What::PhoneBackup)
        );
        assert_eq!(r.folder("Holiday", ["a.jpg"]), None);
        assert_eq!(
            r.file("Ubuntu.VHDX").map(|w| w.0),
            Some(What::VirtualMachineDisk)
        );
        assert_eq!(r.file("notes.txt"), None);
        // Mac-only rows are not used on Windows.
        assert_eq!(r.folder("Pictures.photoslibrary", []), None);
        let mac = table(System::Mac);
        assert_eq!(
            mac.folder("Pictures.photoslibrary", []).map(|w| w.0),
            Some(What::PhotosLibrary)
        );
    }

    #[test]
    fn the_person_s_own_work_and_records_are_named() {
        let r = table(System::Windows);
        for (file, what) in [
            ("Passwords.kdbx", What::PasswordVault),
            ("archive 2019.PST", What::EmailArchive),
            ("ada@example.com.ost", What::EmailCache),
            ("Household.QDF", What::MoneyRecords),
            ("wedding.prproj", What::VideoProject),
            ("track.als", What::MusicProject),
            ("poster.psd", What::DesignFile),
            ("Win11.iso", What::DiscImage),
            ("C-drive.mrimg", What::DiskImageBackup),
        ] {
            assert_eq!(r.file(file).map(|w| w.0), Some(what), "{file}");
        }
        assert_eq!(r.file("phone.txt"), None);
        assert_eq!(
            r.folder("website", ["index.html", ".git"]).map(|w| w.0),
            Some(What::CodeProject)
        );
        assert_eq!(
            r.folder("Notes", [".obsidian", "today.md"]).map(|w| w.0),
            Some(What::NotesVault)
        );
        assert_eq!(
            r.folder("New World", ["level.dat", "region"]).map(|w| w.0),
            Some(What::GameSaves)
        );
        assert_eq!(r.folder("My Games", []).map(|w| w.0), Some(What::GameSaves));
        assert_eq!(
            r.folder("Calibre Library", []).map(|w| w.0),
            Some(What::EbookLibrary)
        );
        // Mac-only rows stay on the Mac.
        assert_eq!(r.file("Installer.dmg"), None);
        let mac = table(System::Mac);
        assert_eq!(
            mac.folder("Song.logicx", []).map(|w| w.0),
            Some(What::MusicProject)
        );
        assert_eq!(mac.folder("My Games", []), None);
    }

    #[test]
    fn traits_come_from_the_kind_rows() {
        let r = Recogniser::shipped();
        for old in [
            What::PhoneBackup,
            What::TimeMachine,
            What::FileHistory,
            What::WindowsImageBackup,
            What::OldWindows,
        ] {
            assert!(old.is_old_backup(), "{old:?}");
        }
        assert!(!What::CameraPhotos.is_old_backup());
        assert!(r.has(What::InstalledGames, Trait::FetchAgain));
        assert!(r.has(What::EmailCache, Trait::FetchAgain));
        assert!(r.has(What::DiscImage, Trait::FetchAgainIfDownloaded));
        assert!(!r.has(What::EmailArchive, Trait::FetchAgain));
        assert_eq!(r.kind(What::PhoneBackup).unwrap().name, "Phone backup");
    }

    /// A newer table never stops an older app: rows it cannot use are skipped and reported, and
    /// every other row still works.
    #[test]
    fn rows_this_version_cannot_use_are_skipped_and_reported() {
        let text = r#"{"format":1,"catalogue":"place-kinds","version":9,"rows":[
            {"kind":{"id":"camera-photos","name":"Camera photos"}},
            {"kind":{"id":"holo-captures","name":"Holo captures","traits":["glows"]}},
            {"marker":{"system":"any","shape":"folder","marker":"DCIM","what":"camera-photos"}},
            {"marker":{"system":"any","shape":"folder","marker":"Holo","what":"holo-captures"}},
            {"marker":{"system":"beos","shape":"folder","marker":"x","what":"camera-photos"}},
            {"marker":{"system":"any","shape":"blob","marker":"x","what":"camera-photos"}},
            {"marker":{"system":"any","shape":"folder","marker":" ","what":"camera-photos"}},
            {"marker":{"system":"any","shape":"folder","marker":"x","what":"Not An Id"}},
            {"teleporter":{"x":1}}
        ]}"#;
        let r = Recogniser::parse(text, System::Windows);
        let (report, unknown) = r.report();
        let skipped: Vec<usize> = report.skipped.iter().map(|s| s.row).collect();
        assert_eq!(skipped, [1, 4, 5, 6, 7, 8]);
        // The Holo marker's kind did not load (its trait is unknown): skipped too.
        assert_eq!(unknown, [(3, What::new("holo-captures").unwrap())]);
        assert_eq!(r.folder("DCIM", []).map(|w| w.0), Some(What::CameraPhotos));
        assert_eq!(r.folder("Holo", []), None);
        // A table that is not a table at all names nothing, and never panics.
        let none = Recogniser::parse("not json", System::Windows);
        assert!(none.report().0.refused.is_some());
        assert_eq!(none.folder("DCIM", []), None);
    }

    #[test]
    fn kind_ids_are_checked() {
        assert_eq!(What::new("camera-photos"), Some(What::CameraPhotos));
        for bad in ["", "-x", "X", "a b", &"a".repeat(32)] {
            assert_eq!(What::new(bad), None, "{bad:?}");
        }
        assert_eq!(What::new(&"a".repeat(31)).unwrap().as_str().len(), 31);
        assert_eq!(
            serde_json::to_string(&What::DiscImage).unwrap(),
            "\"disc-image\""
        );
        assert_eq!(format!("{:?}", What::DiscImage), "What(disc-image)");
    }
}
