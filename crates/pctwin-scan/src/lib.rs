//! Scanning the old laptop (Task List 1.4). It only ever reads.
//!
//! - [`find_special_folders`] asks the system where each special folder really is (Windows known
//!   folders, the macOS standard folders, Linux user-dirs), so a Documents folder moved into
//!   OneDrive, onto the D: drive or into iCloud, or named in another language, is found where it
//!   is. A folder that is not there is reported missing, never guessed.
//! - [`classify`] says whether a folder is on the system drive, another drive or network share, or
//!   inside a cloud storage folder.
//! - [`list_people`] lists everyone with an account on the laptop, by the system's own account ID,
//!   from the list every system keeps for everyone; nobody's files are opened.
//! - [`measure`] counts what a folder holds from folder listings alone: no file is opened, files
//!   that are only in the cloud are counted but never downloaded, links are never followed, and
//!   whatever can't be read is listed. [`Size`] is honest when another person's folder can't be
//!   read without their permission.

mod folders;
mod measure;
mod people;

pub use folders::{
    Facts, FolderLookup, FoundFolder, classify, facts_from_system, find_special_folders,
};
pub use measure::{
    Measure, Size, cloud_only_mac, cloud_only_windows, is_icloud_stub_name, measure,
};
pub use people::{
    Person, list_people, people_from_dscl, people_from_passwd, people_from_profile_list,
    uid_range_from_login_defs,
};
