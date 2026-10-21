//! Scanning the old laptop (the plan). It only ever reads.
//!
//! - [`find_special_folders`] asks the system where each special folder really is (Windows known
//!   folders, the macOS standard folders, Linux user-dirs), so a Documents folder moved into
//!   OneDrive, onto the D: drive or into iCloud, or named in another language, is found where it
//!   is. A folder that is not there is reported missing, never guessed.
//! - [`classify`] says whether a folder is on the system drive, another drive or network share, or
//!   inside a cloud storage folder.
//! - [`census_this_laptop`] counts everyone on the laptop by the system's own account ID, never by
//!   path, in five kinds ([`PersonKind`]): the mover, another person, an account never used, a
//!   leftover from a removed account, and a work or school account. [`list_people`] is everyone
//!   in it with an account and a home folder. Nobody's files are opened.
//! - [`measure`] counts what a folder holds from folder listings alone: no file is opened, files
//!   that are only in the cloud are counted but never downloaded, links are never followed, and
//!   whatever can't be read is listed. [`Size`] is honest when another person's folder can't be
//!   read without their permission.
//! - [`scan_folder`] turns one special folder into move-record items: every file and folder with
//!   its permanent ID, owner and real place; macOS packages as one item; what can't be read or is
//!   only in the cloud is recorded as left out with the reason; photos, videos, music and documents
//!   are counted for the after-move check.
//! - [`list_drives`] lists every real drive with its type and what that type can keep ([`Keeps`]),
//!   and finds another installed system by its own marker files, so the running system is never
//!   assumed to own every drive.
//! - [`scan_this_laptop`] puts it together: the people with honest sizes, the signed-in person's
//!   special folders and the shared one, and the drives, in one move record. Progress is saved
//!   after each folder ([`ScanState`]) so a restart resumes, and [`changes_between`] tells a later
//!   scan what was added, removed or changed.
//! - [`system_drive_health`] reads the old drive's health from what each system reports to a
//!   normal user (never guessed; no answer is Unknown), and [`ReadPlan`] reads a weak drive
//!   carefully.
//! - [`read_usage`] reads what the old laptop records about what this person uses (recent files,
//!   pinned folders) and [`personal_essentials`] matches it to the scanned items, so their own
//!   essentials move first. Read only on the old laptop, used only for the order, sent nowhere.

mod access;
mod acl;
mod census;
mod drives;
mod folders;
mod health;
mod laptop;
mod marks;
mod measure;
mod people;
mod recognise;
mod rules;
mod scan;
mod state;
mod sweep;
mod usage;
mod volumes;
mod whose;
mod worth;

pub use access::{Access, AccessFact, UnixGroups, access_from_mode, access_from_sddl};
pub use acl::{AccessReader, UnixMode, mac_groups_from_dscl, system_reader};
pub use census::{
    Account, Census, CensusFacts, FoundBy, PersonKind, Resident, SystemKind, accounts_from_dscl,
    accounts_from_get_local_user, accounts_from_passwd, census_from, census_this_laptop,
    profile_list_from_reg, service_homes_from_dscl, service_homes_from_passwd, sid_from_whoami,
};
pub use drives::{
    CaseRule, Drive, FileSystem, Keeps, OtherSystem, is_real_mount, list_drives, other_system,
};
pub use folders::{
    Facts, FolderLookup, FoundFolder, classify, facts_from_system, find_special_folders, managed_by,
};
pub use health::{
    Health, ReadPlan, health_from_diskutil, health_from_get_physical_disk, health_from_udisks,
    system_drive_health,
};
pub use laptop::{
    Flow, LaptopScan, PersonSummary, ScanError, build_scan, build_scan_resumable, scan_this_laptop,
};
pub use marks::{MarkSample, has_download_mark, sample_folder};
pub use measure::{
    Measure, Size, cloud_only_mac, cloud_only_windows, is_cloud_only, is_icloud_stub_name, measure,
};
pub use people::{
    Person, list_people, people_from_dscl, people_from_passwd, people_from_profile_list,
    uid_range_from_login_defs,
};
pub use recognise::{Recognised, Recogniser, TableError, What};
pub use rules::{Kind, Row, Rules, RulesError, System, Where};
pub use scan::{Counts, Scan, Stamp, scan_folder, scan_managed_folder};
pub use state::{
    Changes, DoneFolder, NextOwner, STATE_FORMAT, ScanState, StateError, changes_between,
};
pub use sweep::{
    BlindReason, BlindSpot, Fate, FoundPlace, FoundSoFar, Frontier, FrontierView,
    HARD_LINK_CHECK_MIN, ProfileLine, SHALLOW_DEPTH, Space, SpaceLine, Sweep, SweepFacts,
    SweepLimits, TopLine, UNEXPLAINED_FLOOR, VolumeReport, blind_reason_for, sweep,
    sweep_this_laptop,
};
pub use usage::{
    Usage, bookmarks_from_gtk, parse_iso_utc_ns, personal_essentials,
    pinned_from_finder_favourites, pinned_from_quick_access, read_usage, recent_from_lnk_dir,
    recent_from_mdfind, recent_from_xbel,
};
pub use volumes::{DiskKind, Volume, VolumeKind, logical_disks, volume_kind, volumes_this_laptop};
pub use whose::{Facts as PlaceFacts, HomeOf, People, Placement, Rule, Whose, place};
pub use worth::{COPY_MIN_FILES, Worth, look_like_copies, only_on_this_laptop, worth};
