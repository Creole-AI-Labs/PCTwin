//! What each place is (`rules/what.txt`): markers and folder shapes that name a place for the
//! screens (camera photos, a phone backup, an old backup of the laptop), run over the mover's own
//! record after the sweep, so another person's things are never looked at. A label never changes
//! what is included, whose it is or where it lands; it names, orders and weighs.

use std::collections::HashMap;

use pctwin_record::{Item, ItemId, ItemKind, ItemPath, Record};
use serde::{Deserialize, Serialize};

use crate::rules::System;

/// What a place is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum What {
    /// Photos straight from a camera or phone.
    CameraPhotos,
    PhotosLibrary,
    LightroomCatalog,
    MusicLibrary,
    /// An iPhone or iPad backup.
    PhoneBackup,
    TimeMachine,
    FileHistory,
    WindowsImageBackup,
    /// The older copy of Windows an upgrade leaves.
    OldWindows,
    VirtualMachineDisk,
    /// A Linux system kept inside Windows (WSL).
    LinuxOnWindows,
    Containers,
    GameSaves,
    InstalledGames,
    Screenshots,
    /// A backup program's image of a whole drive.
    DiskImageBackup,
    /// An installer or disc image.
    DiscImage,
    /// Mail kept on the laptop, the person's own.
    EmailArchive,
    /// Outlook's offline copy of mail kept on a server: it comes back by itself.
    EmailCache,
    PasswordVault,
    MoneyRecords,
    CodeProject,
    NotesVault,
    VideoProject,
    MusicProject,
    DesignFile,
    EbookLibrary,
}

impl What {
    fn from_text(text: &str) -> Option<Self> {
        Some(match text {
            "camera-photos" => Self::CameraPhotos,
            "photos-library" => Self::PhotosLibrary,
            "lightroom-catalog" => Self::LightroomCatalog,
            "music-library" => Self::MusicLibrary,
            "phone-backup" => Self::PhoneBackup,
            "time-machine" => Self::TimeMachine,
            "file-history" => Self::FileHistory,
            "windows-image-backup" => Self::WindowsImageBackup,
            "old-windows" => Self::OldWindows,
            "virtual-machine-disk" => Self::VirtualMachineDisk,
            "linux-on-windows" => Self::LinuxOnWindows,
            "containers" => Self::Containers,
            "game-saves" => Self::GameSaves,
            "installed-games" => Self::InstalledGames,
            "screenshots" => Self::Screenshots,
            "disk-image-backup" => Self::DiskImageBackup,
            "disc-image" => Self::DiscImage,
            "email-archive" => Self::EmailArchive,
            "email-cache" => Self::EmailCache,
            "password-vault" => Self::PasswordVault,
            "money-records" => Self::MoneyRecords,
            "code-project" => Self::CodeProject,
            "notes-vault" => Self::NotesVault,
            "video-project" => Self::VideoProject,
            "music-project" => Self::MusicProject,
            "design-file" => Self::DesignFile,
            "ebook-library" => Self::EbookLibrary,
            _ => return None,
        })
    }

    /// An old backup of a laptop: worth keeping, but likely the same things again.
    pub fn is_old_backup(self) -> bool {
        matches!(
            self,
            Self::PhoneBackup
                | Self::TimeMachine
                | Self::FileHistory
                | Self::WindowsImageBackup
                | Self::OldWindows
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    Folder,
    FolderSuffix,
    Holds,
    FileSuffix,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Row {
    shape: Shape,
    /// Lower case: names are compared ignoring case.
    marker: String,
    what: What,
    line: usize,
}

/// Why the table could not be read.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("what.txt line {line}: {why}")]
pub struct TableError {
    pub line: usize,
    pub why: String,
}

const TABLE: &str = include_str!("../rules/what.txt");

/// The table, read once, for one system.
#[derive(Debug, Clone)]
pub struct Recogniser {
    rows: Vec<Row>,
}

/// A place the table named: the item, what it is, and the table's line that named it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recognised {
    pub item: ItemId,
    pub what: What,
    pub line: usize,
}

impl Recogniser {
    /// The table shipped with PCTwin, for the system this build runs on.
    pub fn shipped() -> Self {
        // The shipped table is checked by a test; a broken one must never be half-applied.
        Self::parse(TABLE, System::this()).expect("the shipped table reads")
    }

    /// Reads a table for `system`.
    pub fn parse(text: &str, system: System) -> Result<Self, TableError> {
        let mut rows = Vec::new();
        for (i, line) in text.lines().enumerate() {
            let line_no = i + 1;
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let bad = |why: &str| TableError {
                line: line_no,
                why: why.to_string(),
            };
            let fields: Vec<&str> = trimmed.split('|').map(str::trim).collect();
            let [sys, shape, marker, what] = fields[..] else {
                return Err(bad("a row has four parts separated by |"));
            };
            let row_system = match sys {
                "windows" => System::Windows,
                "mac" => System::Mac,
                "linux" => System::Linux,
                "any" => System::Any,
                _ => return Err(bad("unknown system")),
            };
            let shape = match shape {
                "folder" => Shape::Folder,
                "folder-suffix" => Shape::FolderSuffix,
                "holds" => Shape::Holds,
                "file-suffix" => Shape::FileSuffix,
                _ => return Err(bad("unknown shape")),
            };
            if marker.is_empty() {
                return Err(bad("an empty marker"));
            }
            let what = What::from_text(what).ok_or_else(|| bad("unknown kind of place"))?;
            if row_system == System::Any || row_system == system {
                rows.push(Row {
                    shape,
                    marker: marker.to_lowercase(),
                    what,
                    line: line_no,
                });
            }
        }
        Ok(Self { rows })
    }

    /// What a folder named `name`, directly holding `children`, is (the first row that matches).
    pub fn folder<'a>(
        &self,
        name: &str,
        children: impl IntoIterator<Item = &'a str> + Clone,
    ) -> Option<(What, usize)> {
        let name = name.to_lowercase();
        self.rows.iter().find_map(|r| {
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
        self.rows
            .iter()
            .find(|r| r.shape == Shape::FileSuffix && name.ends_with(&r.marker))
            .map(|r| (r.what, r.line))
    }

    /// Every folder and file of `record` the table names. Only the record is read (the mover's
    /// own and shared things), never the drive. A folder inside one already named is not named
    /// again (the photos inside a phone backup are the backup's).
    pub fn record(&self, record: &Record) -> Vec<Recognised> {
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
        kept.into_iter().map(|(_, r)| r).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_table_reads_on_every_system() {
        for system in [System::Windows, System::Mac, System::Linux] {
            let r = Recogniser::parse(TABLE, system).unwrap();
            assert!(r.rows.len() >= 18, "{system:?}");
        }
    }

    /// The table is part of the app: a change to it is a change to the app, made on purpose.
    #[test]
    fn the_shipped_table_is_the_one_reviewed() {
        let fingerprint = blake3::hash(TABLE.replace("\r\n", "\n").as_bytes());
        assert_eq!(
            fingerprint.to_hex().as_str(),
            include_str!("../rules/what.txt.blake3").trim(),
            "rules/what.txt changed: review it, then update rules/what.txt.blake3"
        );
    }

    #[test]
    fn folders_and_files_are_named_by_their_markers() {
        let r = Recogniser::parse(TABLE, System::Windows).unwrap();
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
        let mac = Recogniser::parse(TABLE, System::Mac).unwrap();
        assert_eq!(
            mac.folder("Pictures.photoslibrary", []).map(|w| w.0),
            Some(What::PhotosLibrary)
        );
    }

    #[test]
    fn the_person_s_own_work_and_records_are_named() {
        let r = Recogniser::parse(TABLE, System::Windows).unwrap();
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
        let mac = Recogniser::parse(TABLE, System::Mac).unwrap();
        assert_eq!(
            mac.folder("Song.logicx", []).map(|w| w.0),
            Some(What::MusicProject)
        );
        assert_eq!(mac.folder("My Games", []), None);
    }

    #[test]
    fn a_broken_table_is_refused_whole() {
        for bad in [
            "any | folder | DCIM",
            "beos | folder | DCIM | camera-photos",
            "any | shape | DCIM | camera-photos",
            "any | folder |  | camera-photos",
            "any | folder | DCIM | holiday-snaps",
        ] {
            assert!(Recogniser::parse(bad, System::Windows).is_err(), "{bad}");
        }
    }
}
