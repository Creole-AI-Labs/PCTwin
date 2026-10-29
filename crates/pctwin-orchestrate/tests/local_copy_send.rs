//! Keeping local copies during the move itself (Task List 2.6, wired 29 October 2026): each
//! chosen online-only file is brought down when the send opens it, checked, read, and made online
//! only again; anything that fails is never read and stays safe in its cloud.
// Tests make files to test on.
#![allow(clippy::disallowed_methods)]

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use common::*;
use pctwin_orchestrate::cloud::Os;
use pctwin_orchestrate::local_copy::{
    CloudFiles, Consent, KEEP_FREE_OLD, KeepLocal, Network, Stopped, Wanted, WhyNot,
};
use pctwin_orchestrate::local_copy_send::{KeepFile, KeepOpener};
use pctwin_record::{CloudProvider, ItemId};
use pctwin_transfer::{FsOpener, OnlineOnly, Opener};

const MB: u64 = 1024 * 1024;

fn os() -> Os {
    if cfg!(windows) {
        Os::Windows
    } else if cfg!(target_os = "macos") {
        Os::Mac
    } else {
        Os::Linux
    }
}

#[derive(Default)]
struct Cloud {
    online: BTreeSet<PathBuf>,
    sizes: BTreeMap<PathBuf, u64>,
    free: u64,
    fail_down: BTreeSet<PathBuf>,
    refuse_put_back: BTreeSet<PathBuf>,
    content: BTreeMap<PathBuf, Vec<u8>>,
    /// Its size as the laptop sees it differs from the plan before anything comes down.
    changed: BTreeSet<PathBuf>,
    /// Comes down at another size than it showed.
    grows: BTreeSet<PathBuf>,
    log: Vec<String>,
}

/// The cloud app, shared with the test so it can look afterwards.
#[derive(Clone, Default)]
struct Shared(Arc<Mutex<Cloud>>);

impl Shared {
    fn with<T>(&self, f: impl FnOnce(&mut Cloud) -> T) -> T {
        f(&mut self.0.lock().unwrap())
    }
    fn log(&self) -> Vec<String> {
        self.with(|c| c.log.clone())
    }
}

fn name(p: &Path) -> String {
    p.file_name().unwrap().to_string_lossy().into_owned()
}

impl CloudFiles for Shared {
    fn bring_down(&mut self, path: &Path, size: u64) -> io::Result<()> {
        self.with(|c| {
            if c.sizes[path] + u64::from(c.changed.contains(path)) != size {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "not the size"));
            }
            c.log.push(format!("down {}", name(path)));
            if c.fail_down.contains(path) {
                return Err(io::Error::other("no connection"));
            }
            c.online.remove(path);
            c.free -= c.sizes[path];
            Ok(())
        })
    }
    fn is_online_only(&self, path: &Path) -> io::Result<bool> {
        Ok(self.with(|c| c.online.contains(path)))
    }
    fn open_read(&self, path: &Path) -> io::Result<std::fs::File> {
        std::fs::File::open(path)
    }
    fn size_on_laptop(&self, path: &Path) -> io::Result<u64> {
        Ok(self.with(|c| {
            c.sizes[path] + u64::from(c.changed.contains(path)) + u64::from(c.grows.contains(path))
        }))
    }
    fn put_back(&mut self, path: &Path) -> io::Result<()> {
        self.with(|c| {
            c.log.push(format!("back {}", name(path)));
            if c.refuse_put_back.contains(path) {
                return Err(io::Error::other("refused"));
            }
            c.online.insert(path.to_path_buf());
            c.free += c.sizes[path];
            Ok(())
        })
    }
    fn free_space(&self) -> Option<u64> {
        Some(self.with(|c| c.free))
    }
    fn marked_as_apps_own(&self, _path: &Path) -> io::Result<bool> {
        Ok(false)
    }
    fn head(&self, path: &Path, n: usize) -> io::Result<Vec<u8>> {
        let mut c = self.with(|c| {
            c.content
                .get(path)
                .cloned()
                .unwrap_or_else(|| b"a photo".to_vec())
        });
        c.truncate(n);
        Ok(c)
    }
}

struct Lab {
    _dir: tempfile::TempDir,
    home: PathBuf,
    cloud: Shared,
}

impl Lab {
    fn new(free: u64) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("me");
        std::fs::create_dir_all(home.join("OneDrive")).unwrap();
        let cloud = Shared::default();
        cloud.with(|c| c.free = free);
        Self {
            _dir: dir,
            home,
            cloud,
        }
    }

    /// An online-only file of `size` (as the plan says) in the person's OneDrive folder; its
    /// bytes on disk are its name, so a read can be checked.
    fn file(&self, n: &str, size: u64) -> Wanted {
        let path = self.home.join("OneDrive").join(n);
        std::fs::write(&path, n.as_bytes()).unwrap();
        self.cloud.with(|c| {
            c.online.insert(path.clone());
            c.sizes.insert(path.clone(), size);
        });
        Wanted {
            item: online_only(person("me"), CloudProvider::OneDrive, &[n], size).id,
            account_id: "me".into(),
            provider: CloudProvider::OneDrive,
            path,
            size,
        }
    }

    fn consent(&self, on_metered: bool) -> Consent {
        Consent {
            account_id: "me".into(),
            provider: CloudProvider::OneDrive,
            on_metered,
            home: self.home.clone(),
            root: self.home.join("OneDrive"),
        }
    }

    fn opener(&self, files: &[Wanted], network: Network, on_metered: bool) -> KeepOpener {
        KeepOpener::new(
            Arc::new(FsOpener),
            KeepLocal::new(os(), vec![self.consent(on_metered)], u64::MAX),
            files
                .iter()
                .map(|w| KeepFile {
                    wanted: w.clone(),
                    cloud: 0,
                })
                .collect(),
            vec![Box::new(self.cloud.clone())],
            Box::new(move || network),
        )
    }
}

fn read_all(o: &KeepOpener, p: &Path) -> io::Result<String> {
    let mut s = String::new();
    o.open(p)?.read_to_string(&mut s)?;
    Ok(s)
}

fn safe_in_cloud(e: &io::Error) -> bool {
    e.get_ref().is_some_and(|i| i.is::<OnlineOnly>())
}

#[test]
fn a_chosen_file_comes_down_is_read_and_goes_back_when_the_send_ends() {
    let lab = Lab::new(KEEP_FREE_OLD + 100 * MB);
    let a = lab.file("a.mp4", 10 * MB);
    let o = lab.opener(std::slice::from_ref(&a), Network::Unmetered, false);
    assert_eq!(read_all(&o, &a.path).unwrap(), "a.mp4");
    // Read again (the send checks what the new laptop has): not downloaded twice.
    assert_eq!(read_all(&o, &a.path).unwrap(), "a.mp4");
    assert_eq!(lab.cloud.log(), ["down a.mp4"]);
    let report = o.finish(&|i| i == a.item);
    assert_eq!(lab.cloud.log(), ["down a.mp4", "back a.mp4"]);
    let f = &report.files[&a.item];
    assert!(f.sent);
    assert_eq!(f.why_not, None);
    assert_eq!(f.back_online_only, Some(true));
    assert_eq!(report.downloaded_bytes, 10 * MB);
    assert_eq!(report.still_on_old_laptop_bytes, 0);
    assert_eq!(report.words_for(&a.item), None);
}

#[test]
fn a_file_the_new_laptop_did_not_get_is_not_said_sent() {
    let lab = Lab::new(KEEP_FREE_OLD + 100 * MB);
    let a = lab.file("a.mp4", 10 * MB);
    let o = lab.opener(std::slice::from_ref(&a), Network::Unmetered, false);
    read_all(&o, &a.path).unwrap();
    let report = o.finish(&|_| false);
    let f = &report.files[&a.item];
    assert!(!f.sent);
    assert_eq!(f.why_not, Some(WhyNot::NotSent));
    assert_eq!(f.back_online_only, Some(true));
}

#[test]
fn a_file_that_did_not_come_down_is_never_read() {
    let lab = Lab::new(KEEP_FREE_OLD + 100 * MB);
    let a = lab.file("a.mp4", 10 * MB);
    lab.cloud.with(|c| c.fail_down.insert(a.path.clone()));
    let o = lab.opener(std::slice::from_ref(&a), Network::Unmetered, false);
    let e = read_all(&o, &a.path).unwrap_err();
    assert!(safe_in_cloud(&e), "{e}");
    let r = o.finish(&|_| true);
    assert_eq!(r.files[&a.item].why_not, Some(WhyNot::DownloadFailed));
    assert!(!r.files[&a.item].sent);
    assert_eq!(r.downloaded_bytes, 0);
}

#[test]
fn space_is_kept_free_and_finished_files_go_back_to_make_room() {
    // Room for one 60 MB file at a time, with the room kept free.
    let lab = Lab::new(KEEP_FREE_OLD + 80 * MB);
    let a = lab.file("a.mp4", 60 * MB);
    let b = lab.file("b.mp4", 60 * MB);
    let big = lab.file("big.mp4", 81 * MB);
    let o = lab.opener(
        &[a.clone(), b.clone(), big.clone()],
        Network::Unmetered,
        false,
    );
    read_all(&o, &a.path).unwrap();
    read_all(&o, &b.path).unwrap();
    assert_eq!(lab.cloud.log(), ["down a.mp4", "back a.mp4", "down b.mp4"]);
    // Never more down than the room: the drive keeps its spare space.
    assert!(lab.cloud.with(|c| c.free) >= KEEP_FREE_OLD);
    let e = read_all(&o, &big.path).unwrap_err();
    assert!(safe_in_cloud(&e));
    let r = o.finish(&|_| true);
    assert_eq!(r.files[&big.item].why_not, Some(WhyNot::TooBig));
    assert!(r.files[&a.item].sent && r.files[&b.item].sent);
    assert!(!lab.cloud.log().contains(&"down big.mp4".to_string()));
}

#[test]
fn a_file_still_being_read_is_never_put_back_for_room() {
    let lab = Lab::new(KEEP_FREE_OLD + 80 * MB);
    let a = lab.file("a.mp4", 60 * MB);
    let b = lab.file("b.mp4", 60 * MB);
    let o = lab.opener(&[a.clone(), b.clone()], Network::Unmetered, false);
    let reading = o.open(&a.path).unwrap();
    let e = read_all(&o, &b.path).unwrap_err();
    assert!(safe_in_cloud(&e));
    assert_eq!(lab.cloud.log(), ["down a.mp4"]);
    drop(reading);
    let r = o.report();
    assert_eq!(r.stopped, Some(Stopped::NoSpace));
    assert_eq!(r.files[&b.item].why_not, Some(WhyNot::Paused));
    // Once it is done, the next one fits again.
    assert_eq!(read_all(&o, &b.path).unwrap(), "b.mp4");
}

#[test]
fn nothing_comes_down_on_a_connection_that_may_charge_unless_allowed() {
    let lab = Lab::new(KEEP_FREE_OLD + 100 * MB);
    let a = lab.file("a.mp4", 10 * MB);
    for network in [Network::Metered, Network::NotKnown] {
        let o = lab.opener(std::slice::from_ref(&a), network, false);
        assert!(safe_in_cloud(&read_all(&o, &a.path).unwrap_err()));
        let r = o.finish(&|_| true);
        assert_eq!(r.stopped, Some(Stopped::MeteredNow));
        assert_eq!(r.files[&a.item].why_not, Some(WhyNot::Paused));
    }
    assert!(lab.cloud.log().is_empty());
    let allowed = lab.opener(std::slice::from_ref(&a), Network::Metered, true);
    assert_eq!(read_all(&allowed, &a.path).unwrap(), "a.mp4");
}

#[test]
fn account_state_or_a_changed_file_is_put_back_and_never_read() {
    let lab = Lab::new(KEEP_FREE_OLD + 100 * MB);
    let token = lab.file("notes.json", MB);
    let changed = lab.file("c.mp4", MB);
    lab.cloud.with(|c| {
        c.content
            .insert(token.path.clone(), br#"{"refresh_token":"abc"}"#.to_vec());
        c.grows.insert(changed.path.clone());
    });
    let o = lab.opener(&[token.clone(), changed.clone()], Network::Unmetered, false);
    assert!(safe_in_cloud(&read_all(&o, &token.path).unwrap_err()));
    assert!(safe_in_cloud(&read_all(&o, &changed.path).unwrap_err()));
    let r = o.finish(&|_| true);
    assert_eq!(r.files[&token.item].why_not, Some(WhyNot::OutOfScope));
    assert_eq!(r.files[&changed.item].why_not, Some(WhyNot::Changed));
    assert_eq!(r.files[&token.item].back_online_only, Some(true));
    assert_eq!(r.files[&changed.item].back_online_only, Some(true));
    assert_eq!(
        lab.cloud.log(),
        [
            "down notes.json",
            "back notes.json",
            "down c.mp4",
            "back c.mp4"
        ]
    );
}

/// A file whose size on the laptop is not the approved one is refused before anything comes
/// down (second security review): it changed in the cloud since the plan.
#[test]
fn a_file_not_the_approved_size_is_never_downloaded() {
    let lab = Lab::new(KEEP_FREE_OLD + 100 * MB);
    let a = lab.file("a.mp4", MB);
    lab.cloud.with(|c| c.changed.insert(a.path.clone()));
    let o = lab.opener(std::slice::from_ref(&a), Network::Unmetered, false);
    assert!(safe_in_cloud(&read_all(&o, &a.path).unwrap_err()));
    let r = o.finish(&|_| true);
    assert_eq!(r.files[&a.item].why_not, Some(WhyNot::Changed));
    assert_eq!(
        r.files[&a.item].back_online_only, None,
        "never brought down"
    );
    assert!(lab.cloud.log().is_empty());
    assert_eq!(r.downloaded_bytes, 0);
}

#[test]
fn a_file_the_cloud_app_keeps_down_is_said_with_its_space() {
    let lab = Lab::new(KEEP_FREE_OLD + 100 * MB);
    let a = lab.file("a.mp4", 10 * MB);
    lab.cloud.with(|c| c.refuse_put_back.insert(a.path.clone()));
    let o = lab.opener(std::slice::from_ref(&a), Network::Unmetered, false);
    read_all(&o, &a.path).unwrap();
    let r = o.finish(&|_| true);
    assert_eq!(r.files[&a.item].back_online_only, Some(false));
    assert_eq!(r.still_on_old_laptop_bytes, 10 * MB);
    assert!(r.words_for(&a.item).unwrap().contains("still takes space"));
}

#[test]
fn files_not_chosen_or_outside_the_cloud_folder_are_left_alone() {
    let lab = Lab::new(KEEP_FREE_OLD + 100 * MB);
    let plain = lab.home.join("plain.txt");
    std::fs::write(&plain, b"plain").unwrap();
    // Chosen, but outside the person's cloud folder: never brought down.
    let outside_path = lab.home.join("outside.mp4");
    std::fs::write(&outside_path, b"x").unwrap();
    lab.cloud.with(|c| {
        c.online.insert(outside_path.clone());
        c.sizes.insert(outside_path.clone(), MB);
    });
    let outside = Wanted {
        item: ItemId::from_hex(&"ab".repeat(16)).unwrap(),
        account_id: "me".into(),
        provider: CloudProvider::OneDrive,
        path: outside_path.clone(),
        size: MB,
    };
    // Someone else's file: no consent of theirs.
    let theirs = Wanted {
        account_id: "them".into(),
        ..lab.file("theirs.mp4", MB)
    };
    let o = lab.opener(
        &[outside.clone(), theirs.clone()],
        Network::Unmetered,
        false,
    );
    assert_eq!(read_all(&o, &plain).unwrap(), "plain");
    assert!(safe_in_cloud(&read_all(&o, &outside_path).unwrap_err()));
    assert!(safe_in_cloud(&read_all(&o, &theirs.path).unwrap_err()));
    let r = o.finish(&|_| true);
    assert_eq!(r.files[&outside.item].why_not, Some(WhyNot::OutOfScope));
    assert_eq!(r.files[&theirs.item].why_not, Some(WhyNot::NoConsent));
    assert!(lab.cloud.log().is_empty());
}

#[test]
fn a_file_already_on_the_laptop_is_read_as_any_other() {
    let lab = Lab::new(KEEP_FREE_OLD + 100 * MB);
    let a = lab.file("a.mp4", 10 * MB);
    lab.cloud.with(|c| c.online.remove(&a.path));
    let o = lab.opener(std::slice::from_ref(&a), Network::Unmetered, false);
    assert_eq!(read_all(&o, &a.path).unwrap(), "a.mp4");
    let r = o.finish(&|_| true);
    assert_eq!(r.files[&a.item].why_not, Some(WhyNot::AlreadyOnLaptop));
    assert!(lab.cloud.log().is_empty());
}

mod durable {
    use super::*;
    use pctwin_orchestrate::local_copy_send::{KeptList, ListStore, put_back_listed};

    /// The list in a file, as the app keeps it.
    struct FileStore(PathBuf);

    impl ListStore for FileStore {
        fn read(&self) -> io::Result<Option<Vec<u8>>> {
            match std::fs::read(&self.0) {
                Ok(b) => Ok(Some(b)),
                Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
                Err(e) => Err(e),
            }
        }
        fn write(&self, bytes: &[u8]) -> io::Result<()> {
            std::fs::write(&self.0, bytes)
        }
        fn remove(&self) -> io::Result<()> {
            match std::fs::remove_file(&self.0) {
                Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
                _ => Ok(()),
            }
        }
    }

    fn store(list: &Path) -> Box<dyn ListStore> {
        Box::new(FileStore(list.to_path_buf()))
    }

    fn listed(lab: &Lab, list: &Path, files: &[Wanted]) -> KeepOpener {
        let keep: Vec<KeepFile> = files
            .iter()
            .map(|w| KeepFile {
                wanted: w.clone(),
                cloud: 0,
            })
            .collect();
        let made = KeptList::create(store(list), &keep, &[lab.home.join("OneDrive")]).unwrap();
        lab.opener(files, Network::Unmetered, false).recorded(made)
    }

    fn down_in(list: &Path) -> Vec<(String, bool)> {
        let v: serde_json::Value = serde_json::from_slice(&std::fs::read(list).unwrap()).unwrap();
        v["files"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| {
                (
                    name(Path::new(f["path"].as_str().unwrap())),
                    f["down"].as_bool().unwrap(),
                )
            })
            .collect()
    }

    /// The old laptop ends as it was: a move stopped part way leaves the list, and the next start
    /// puts every file it names back online only, then removes it.
    #[test]
    fn a_move_stopped_part_way_leaves_the_list_and_the_next_start_puts_every_file_back() {
        let lab = Lab::new(KEEP_FREE_OLD + 100 * MB);
        let a = lab.file("a.mp4", 10 * MB);
        let b = lab.file("b.mp4", 10 * MB);
        let list = lab.home.join("kept-local.json");
        let o = listed(&lab, &list, &[a.clone(), b.clone()]);
        // Written before anything came down, with each file's state before.
        assert_eq!(
            down_in(&list),
            [("a.mp4".to_string(), false), ("b.mp4".to_string(), false)]
        );
        assert!(
            std::fs::read_to_string(&list)
                .unwrap()
                .contains("online-only")
        );
        read_all(&o, &a.path).unwrap();
        assert_eq!(
            down_in(&list),
            [("a.mp4".to_string(), true), ("b.mp4".to_string(), false)]
        );
        // Stopped here: PCTwin closed, nothing put back.
        drop(o);
        assert!(!lab.cloud.with(|c| c.online.contains(&a.path)));
        let cloud = lab.cloud.clone();
        let done = put_back_listed(
            store(&list).as_ref(),
            &[lab.home.join("OneDrive")],
            &mut |root| {
                assert_eq!(root, lab.home.join("OneDrive"));
                Box::new(cloud.clone())
            },
        )
        .unwrap();
        assert_eq!(done.back, std::slice::from_ref(&a.path));
        assert!(done.still_down.is_empty());
        assert!(lab.cloud.with(|c| c.online.contains(&a.path)));
        assert_eq!(lab.cloud.log(), ["down a.mp4", "back a.mp4"]);
        assert!(!list.exists(), "the list is removed");
        // Nothing left: a second start does nothing.
        assert_eq!(
            put_back_listed(
                store(&list).as_ref(),
                &[lab.home.join("OneDrive")],
                &mut |_| Box::new(cloud.clone())
            )
            .unwrap(),
            Default::default()
        );
    }

    #[test]
    fn a_finished_move_removes_its_list_and_one_the_cloud_app_kept_down_stays_for_next_time() {
        let lab = Lab::new(KEEP_FREE_OLD + 100 * MB);
        let a = lab.file("a.mp4", 10 * MB);
        let list = lab.home.join("kept-local.json");
        let o = listed(&lab, &list, std::slice::from_ref(&a));
        read_all(&o, &a.path).unwrap();
        o.finish(&|_| true);
        assert!(!list.exists());

        let b = lab.file("b.mp4", 10 * MB);
        lab.cloud.with(|c| c.refuse_put_back.insert(b.path.clone()));
        let o = listed(&lab, &list, std::slice::from_ref(&b));
        read_all(&o, &b.path).unwrap();
        o.finish(&|_| true);
        assert_eq!(down_in(&list), [("b.mp4".to_string(), true)]);
        // Still refused at the next start: kept on the list.
        let cloud = lab.cloud.clone();
        let done = put_back_listed(
            store(&list).as_ref(),
            &[lab.home.join("OneDrive")],
            &mut |_| Box::new(cloud.clone()),
        )
        .unwrap();
        assert_eq!(done.still_down, std::slice::from_ref(&b.path));
        assert!(list.exists());
        // The start after that, it goes back, and the list is gone.
        lab.cloud.with(|c| c.refuse_put_back.clear());
        let done = put_back_listed(
            store(&list).as_ref(),
            &[lab.home.join("OneDrive")],
            &mut |_| Box::new(cloud.clone()),
        )
        .unwrap();
        assert_eq!(done.back, std::slice::from_ref(&b.path));
        assert!(!list.exists());
    }

    /// The cloud folders are found again at the start: a file the list places in a folder that
    /// is not one of them is never touched (the list could have been changed).
    #[test]
    fn a_left_list_is_put_back_only_inside_the_cloud_folders_found_now() {
        let lab = Lab::new(KEEP_FREE_OLD + 100 * MB);
        let a = lab.file("a.mp4", 10 * MB);
        let list = lab.home.join("kept-local.json");
        let o = listed(&lab, &list, std::slice::from_ref(&a));
        read_all(&o, &a.path).unwrap();
        drop(o);
        let cloud = lab.cloud.clone();
        let elsewhere = [lab.home.join("Elsewhere")];
        let done = put_back_listed(store(&list).as_ref(), &elsewhere, &mut |_| {
            Box::new(cloud.clone())
        })
        .unwrap();
        assert_eq!(done.still_down, std::slice::from_ref(&a.path));
        assert_eq!(lab.cloud.log(), ["down a.mp4"], "not touched");
        assert!(list.exists());
    }

    #[test]
    fn nothing_comes_down_unless_it_is_written_down_first() {
        let lab = Lab::new(KEEP_FREE_OLD + 100 * MB);
        let a = lab.file("a.mp4", 10 * MB);
        let dir = lab.home.join("pctwin-data");
        std::fs::create_dir_all(&dir).unwrap();
        let list = dir.join("kept-local.json");
        let o = listed(&lab, &list, std::slice::from_ref(&a));
        // The list can no longer be written: the file is never brought down.
        std::fs::remove_file(&list).unwrap();
        std::fs::remove_dir(&dir).unwrap();
        assert!(safe_in_cloud(&read_all(&o, &a.path).unwrap_err()));
        assert!(lab.cloud.log().is_empty());
        assert_eq!(
            o.report().files[&a.item].why_not,
            Some(WhyNot::DownloadFailed)
        );
    }

    #[test]
    fn an_earlier_list_is_never_overwritten_and_a_strange_one_is_never_acted_on() {
        let lab = Lab::new(KEEP_FREE_OLD + 100 * MB);
        let list = lab.home.join("kept-local.json");
        std::fs::write(&list, b"not a list").unwrap();
        assert!(KeptList::create(store(&list), &[], &[]).is_err());
        let cloud = lab.cloud.clone();
        assert!(
            put_back_listed(
                store(&list).as_ref(),
                &[lab.home.join("OneDrive")],
                &mut |_| Box::new(cloud.clone())
            )
            .is_err()
        );
        assert!(list.exists(), "kept, never acted on");
        assert!(lab.cloud.log().is_empty());
    }
}
