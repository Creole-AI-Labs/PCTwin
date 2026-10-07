//! Scanning the old laptop (Task List 1.4). It only ever reads.
//!
//! - [`find_special_folders`] asks the system where each special folder really is (Windows known
//!   folders, the macOS standard folders, Linux user-dirs), so a Documents folder moved into
//!   OneDrive, onto the D: drive or into iCloud, or named in another language, is found where it
//!   is. A folder that is not there is reported missing, never guessed.
//! - [`classify`] says whether a folder is on the system drive, another drive or network share, or
//!   inside a cloud storage folder.

mod folders;

pub use folders::{
    Facts, FolderLookup, FoundFolder, classify, facts_from_system, find_special_folders,
};
