//! The old laptop: keeping a local copy of the mover's online-only files, with their consent
//! (Task List 2.6; the brief's item A, 29 October 2026). The person chose clouds on the choose
//! page; here their files of those clouds are matched to the cloud folder they are in (found on
//! the laptop, for any sync app), and the send reads them through
//! [`pctwin_orchestrate::local_copy_send::KeepOpener`]: brought down one at a time within the
//! old laptop's free space, sent, and made online only again. A file that cannot be matched to
//! its cloud folder is not brought down: it stays out of the move as stored online only.
//!
//! The old laptop is read only, except for this step: consented, recorded in the outcome
//! ([`KeptOnlineView`]) and reversed (each file made online only again).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use pctwin_orchestrate::local_copy::{
    Consent, KeepLocal, Network, Report, Stopped, Wanted, WhyNot,
};
use pctwin_orchestrate::local_copy_send::{KeepFile, KeepOpener};
use pctwin_record::{CloudProvider, Item, ItemId, Record};
use serde::Serialize;

use crate::moving::KeepLocalAnswer;

/// Most bytes brought down at once is the old laptop's free space (kept per file).
const NO_BATCH_LIMIT: u64 = u64::MAX;
/// How long the cloud app is given to make a file online only again before PCTwin says what it
/// sees.
const PUT_BACK_WAIT: Duration = Duration::from_secs(10);

/// One cloud folder of the mover's on this laptop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloudFolder {
    pub provider: CloudProvider,
    pub root: PathBuf,
}

/// The mover's cloud folders on this laptop, found by the scan's general rules (any sync app).
pub fn cloud_folders(home: &Path) -> Vec<CloudFolder> {
    use pctwin_scan::clouds;
    clouds::find_clouds(
        clouds::shipped(),
        clouds::System::this(),
        home,
        clouds::seen_on_this_laptop(home),
        &clouds::SystemRoutes,
        &|p| p.is_dir(),
    )
    .into_iter()
    .filter(|c| !c.work)
    .map(|c| CloudFolder {
        provider: c.provider,
        root: c.root,
    })
    .collect()
}

/// The files to keep a copy of, each with the cloud folder holding it (an index into `folders`):
/// only items in `ids`, of a consented cloud, whose file is inside one of that cloud's folders.
/// Returns them, and the items that could not be matched (they go back to stored online only).
pub fn match_files(
    record: &Record,
    ids: &[ItemId],
    mover: &str,
    folders: &[CloudFolder],
    path_of: &dyn Fn(&Item) -> Option<PathBuf>,
) -> (Vec<KeepFile>, Vec<ItemId>) {
    let mut files = Vec::new();
    let mut unmatched = Vec::new();
    for item in record.items.iter().filter(|i| ids.contains(&i.id)) {
        let pctwin_record::Storage::Cloud { provider } = item.place.storage else {
            unmatched.push(item.id);
            continue;
        };
        let path = path_of(item);
        let cloud = path.as_ref().and_then(|p| {
            folders
                .iter()
                .enumerate()
                .filter(|(_, f)| f.provider == provider && p.starts_with(&f.root))
                .max_by_key(|(_, f)| f.root.as_os_str().len())
                .map(|(n, _)| n)
        });
        match (path, cloud) {
            (Some(path), Some(cloud)) => files.push(KeepFile {
                wanted: Wanted {
                    item: item.id,
                    account_id: mover.to_string(),
                    provider,
                    path,
                    size: item.size_bytes,
                },
                cloud,
            }),
            _ => unmatched.push(item.id),
        }
    }
    (files, unmatched)
}

/// The consents: one per cloud folder of a cloud the person chose.
pub fn consents(
    answers: &[KeepLocalAnswer],
    mover: &str,
    home: &Path,
    folders: &[CloudFolder],
) -> Vec<Consent> {
    folders
        .iter()
        .filter_map(|f| {
            let answer = answers.iter().find(|a| a.provider == f.provider)?;
            Some(Consent {
                account_id: mover.to_string(),
                provider: f.provider,
                on_metered: answer.on_metered,
                home: home.to_path_buf(),
                root: f.root.clone(),
            })
        })
        .collect()
}

/// The connection now, as the system says (Windows); not known elsewhere, which counts as one
/// that may charge.
fn network_now() -> Network {
    match pctwin_apps::fetch::network_cost() {
        pctwin_apps::fetch::NetworkCost::Unmetered => Network::Unmetered,
        pctwin_apps::fetch::NetworkCost::Metered => Network::Metered,
        pctwin_apps::fetch::NetworkCost::Unknown => Network::NotKnown,
    }
}

/// The send's opener for the kept files (Windows' cloud files support), over `inner`.
#[cfg(windows)]
pub fn opener(
    inner: Arc<dyn pctwin_transfer::Opener>,
    consents: Vec<Consent>,
    files: Vec<KeepFile>,
    folders: &[CloudFolder],
) -> KeepOpener {
    let system32 = pctwin_helper_os::system_folder(pctwin_helper_os::SystemFolder::Windows)
        .ok()
        .map(|w| w.join("System32"));
    let clouds: Vec<Box<dyn pctwin_orchestrate::local_copy::CloudFiles + Send>> = folders
        .iter()
        .map(|f| {
            Box::new(pctwin_orchestrate::local_copy::WindowsCloudFiles::new(
                &f.root,
                PUT_BACK_WAIT,
                system32.as_deref(),
            )) as Box<dyn pctwin_orchestrate::local_copy::CloudFiles + Send>
        })
        .collect();
    KeepOpener::new(
        inner,
        KeepLocal::new(
            pctwin_orchestrate::cloud::Os::Windows,
            consents,
            NO_BATCH_LIMIT,
        ),
        files,
        clouds,
        Box::new(network_now),
    )
}

/// The durable list of files a move made local, in PCTwin's own data folder.
pub fn list_path<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Option<PathBuf> {
    Some(crate::moving::journal_path(app)?.with_file_name("kept-local.json"))
}

/// The list kept in one file in PCTwin's own data folder: written whole beside it, flushed, then
/// put in its place.
pub struct FileList(pub PathBuf);

impl pctwin_orchestrate::local_copy_send::ListStore for FileList {
    fn read(&self) -> std::io::Result<Option<Vec<u8>>> {
        match std::fs::read(&self.0) {
            Ok(b) => Ok(Some(b)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }
    fn write(&self, bytes: &[u8]) -> std::io::Result<()> {
        use std::io::Write;
        let mut tmp = self.0.as_os_str().to_owned();
        tmp.push(".new");
        let tmp = PathBuf::from(tmp);
        {
            let mut f = std::fs::File::create(&tmp)?;
            f.write_all(bytes)?;
            f.sync_all()?;
        }
        std::fs::rename(&tmp, &self.0)
    }
    fn remove(&self) -> std::io::Result<()> {
        match std::fs::remove_file(&self.0) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }
}

/// At PCTwin's start and before a move: puts back online only every file an earlier move made
/// local and did not put back, then removes the list (Windows; elsewhere no list is ever made).
pub fn put_back_left<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    #[cfg(windows)]
    if let Some(list) = list_path(app) {
        let system32 = pctwin_helper_os::system_folder(pctwin_helper_os::SystemFolder::Windows)
            .ok()
            .map(|w| w.join("System32"));
        let _ =
            pctwin_orchestrate::local_copy_send::put_back_listed(&FileList(list), &mut |root| {
                Box::new(pctwin_orchestrate::local_copy::WindowsCloudFiles::new(
                    root,
                    PUT_BACK_WAIT,
                    system32.as_deref(),
                ))
            });
    }
    #[cfg(not(windows))]
    let _ = app;
}

/// Why keeping copies paused, for the outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum KeptPaused {
    /// The connection may charge for data, and the person did not allow that.
    MeteredNow,
    /// The old laptop had no room to download more.
    NoSpace,
}

/// What keeping copies did on the old laptop, for its outcome page: what was downloaded, what was
/// copied, what stayed safe in its cloud, and what the cloud app has not made online only again
/// (it still takes space on the old laptop: "right-click it and choose Free up space").
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeptOnlineView {
    pub copied: u64,
    pub safe_in_cloud: u64,
    pub downloaded_bytes: u64,
    pub still_on_old_laptop_files: u64,
    pub still_on_old_laptop_bytes: u64,
    pub paused: Option<KeptPaused>,
}

impl KeptOnlineView {
    pub fn of(report: &Report) -> Self {
        let copied = report.files.values().filter(|f| f.sent).count() as u64;
        Self {
            copied,
            safe_in_cloud: report
                .files
                .values()
                .filter(|f| !f.sent && f.why_not != Some(WhyNot::AlreadyOnLaptop))
                .count() as u64,
            downloaded_bytes: report.downloaded_bytes,
            still_on_old_laptop_files: report
                .files
                .values()
                .filter(|f| f.back_online_only == Some(false))
                .count() as u64,
            still_on_old_laptop_bytes: report.still_on_old_laptop_bytes,
            paused: report.stopped.map(|s| match s {
                Stopped::MeteredNow => KeptPaused::MeteredNow,
                Stopped::NoSpace => KeptPaused::NoSpace,
            }),
        }
    }
}

/// Puts the items in `unmatched` back as stored online only (not in the move).
pub fn leave_unmatched(record: &mut Record, unmatched: &[ItemId]) {
    let unmatched: BTreeSet<&ItemId> = unmatched.iter().collect();
    for item in &mut record.items {
        if unmatched.contains(&item.id) {
            item.inclusion = pctwin_record::Inclusion::LeftOut {
                reason: pctwin_record::LeftOutReason::CloudOnly,
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pctwin_orchestrate::local_copy::FileReport;

    #[test]
    fn the_outcome_counts_what_was_copied_safe_and_still_down() {
        let id = |n: u8| ItemId::from_hex(&format!("{n:02x}").repeat(16)).unwrap();
        let file = |sent, back, why| FileReport {
            provider: CloudProvider::OneDrive,
            sent,
            back_online_only: back,
            why_not: why,
            size: 10,
        };
        let mut r = Report::default();
        r.files.insert(id(1), file(true, Some(true), None));
        r.files.insert(id(2), file(true, Some(false), None));
        r.files
            .insert(id(3), file(false, None, Some(WhyNot::DownloadFailed)));
        r.files
            .insert(id(4), file(false, None, Some(WhyNot::AlreadyOnLaptop)));
        r.downloaded_bytes = 20;
        r.still_on_old_laptop_bytes = 10;
        r.stopped = Some(Stopped::NoSpace);
        assert_eq!(
            KeptOnlineView::of(&r),
            KeptOnlineView {
                copied: 2,
                safe_in_cloud: 1,
                downloaded_bytes: 20,
                still_on_old_laptop_files: 1,
                still_on_old_laptop_bytes: 10,
                paused: Some(KeptPaused::NoSpace),
            }
        );
    }

    #[test]
    fn files_are_matched_to_their_own_cloud_folder_only() {
        use pctwin_record::{FolderRole, ItemPath, Place, Storage};
        let home = PathBuf::from(if cfg!(windows) {
            r"C:\Users\me"
        } else {
            "/home/me"
        });
        let folders = vec![
            CloudFolder {
                provider: CloudProvider::OneDrive,
                root: home.join("OneDrive"),
            },
            CloudFolder {
                provider: CloudProvider::Dropbox,
                root: home.join("Dropbox"),
            },
        ];
        let laptop = pctwin_record::LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap();
        let mk = |provider, name: &str| {
            let owner = pctwin_record::Owner::Person {
                account_id: "me".into(),
            };
            let place = Place {
                role: FolderRole::Documents,
                storage: Storage::Cloud { provider },
            };
            let path = ItemPath::new(vec![pctwin_record::ItemName::from_text(name)]).unwrap();
            pctwin_record::Item {
                id: ItemId::derive(&laptop, &owner, &place, &path),
                kind: pctwin_record::ItemKind::File,
                owner,
                place,
                path,
                size_bytes: 10,
                modified_ns: None,
                portability: pctwin_record::Portability::Portable,
                managed_by: pctwin_record::ManagedBy::Personal,
                download_bytes: None,
                inclusion: pctwin_record::Inclusion::LeftOut {
                    reason: pctwin_record::LeftOutReason::CloudOnly,
                },
            }
        };
        let a = mk(CloudProvider::OneDrive, "a.mp4");
        let b = mk(CloudProvider::Dropbox, "b.mp4");
        let c = mk(CloudProvider::OneDrive, "c.mp4");
        let mut record = Record::new(laptop);
        record.items = vec![a.clone(), b.clone(), c.clone()];
        let path_of = |i: &Item| -> Option<PathBuf> {
            match i.id {
                x if x == a.id => Some(home.join("OneDrive").join("a.mp4")),
                // In the Dropbox folder, but said to be OneDrive's: not matched.
                x if x == b.id => Some(home.join("Dropbox").join("b.mp4")),
                _ => None,
            }
        };
        let mut b_as_onedrive = record.clone();
        b_as_onedrive.items[1].place.storage = Storage::Cloud {
            provider: CloudProvider::OneDrive,
        };
        let (files, unmatched) = match_files(
            &b_as_onedrive,
            &[a.id, b.id, c.id],
            "me",
            &folders,
            &path_of,
        );
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].wanted.item, a.id);
        assert_eq!(files[0].cloud, 0);
        assert_eq!(unmatched, vec![b.id, c.id]);
        // Its own cloud's folder: matched.
        let (files, _) = match_files(&record, &[b.id], "me", &folders, &path_of);
        assert_eq!(files[0].cloud, 1);
        // Only the clouds chosen get a consent.
        let answers = [KeepLocalAnswer {
            provider: CloudProvider::Dropbox,
            on_metered: true,
        }];
        let c = consents(&answers, "me", &home, &folders);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].root, home.join("Dropbox"));
        assert!(c[0].on_metered);
    }
}
