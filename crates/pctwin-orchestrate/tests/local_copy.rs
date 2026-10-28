//! Keep a local copy too, with consent (Task List 2.6; acceptance T26-013, T26-014): online-only
//! files are downloaded on the old laptop in bounded batches after a space check, sent, then
//! returned to online only, and every effect on the cloud app is recorded and said plainly.
// Tests make files to test on.
#![allow(clippy::disallowed_methods)]

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Path, PathBuf};

use common::*;
use pctwin_orchestrate::cloud::Os;
use pctwin_orchestrate::local_copy::{
    Batch, CloudFiles, Consent, KEEP_FREE_OLD, KeepLocal, Network, Stopped, Wanted, WhyNot,
    plan_batches,
};
use pctwin_record::{CloudProvider, ItemId};

const GB: u64 = 1024 * 1024 * 1024;

/// A cloud folder in memory: which files are online only, how much space is free, and what
/// fails.
#[derive(Default)]
struct Fake {
    online: BTreeSet<PathBuf>,
    sizes: BTreeMap<PathBuf, u64>,
    free: u64,
    fail_down: BTreeSet<PathBuf>,
    /// Brought down, but the cloud app leaves it online only (a broken download).
    stays_online: BTreeSet<PathBuf>,
    refuse_put_back: BTreeSet<PathBuf>,
    /// The cloud app accepts the request but does not make it online only in time.
    slow_put_back: BTreeSet<PathBuf>,
    /// Marked by the system as the app's own (hidden and system).
    apps_own: BTreeSet<PathBuf>,
    /// What a file holds, when a test says (otherwise plain words).
    content: BTreeMap<PathBuf, Vec<u8>>,
    /// Files whose first bytes cannot be read.
    fail_head: BTreeSet<PathBuf>,
    log: Vec<String>,
}

/// The person's home folder on the old laptop, and its system.
fn home() -> PathBuf {
    PathBuf::from(if cfg!(windows) {
        r"C:\Users\me"
    } else {
        "/home/me"
    })
}

fn os() -> Os {
    if cfg!(windows) {
        Os::Windows
    } else if cfg!(target_os = "macos") {
        Os::Mac
    } else {
        Os::Linux
    }
}

/// A file in the person's OneDrive folder.
fn at(p: &str) -> PathBuf {
    home().join("OneDrive").join(p)
}

/// How the log names a file: its path below the OneDrive folder.
fn shown(path: &Path) -> String {
    path.strip_prefix(home().join("OneDrive"))
        .unwrap_or(path)
        .display()
        .to_string()
}

impl Fake {
    fn add(&mut self, p: &str, size: u64) -> (ItemId, PathBuf, u64) {
        let path = at(p);
        self.online.insert(path.clone());
        self.sizes.insert(path.clone(), size);
        // The item's own name: the path with anything but letters and digits made plain.
        let name: String = p
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();
        let id = online_only(person("me"), CloudProvider::OneDrive, &[&name], size).id;
        (id, path, size)
    }
}

impl CloudFiles for Fake {
    fn bring_down(&mut self, path: &Path) -> io::Result<()> {
        self.log.push(format!("down {}", shown(path)));
        if self.fail_down.contains(path) {
            return Err(io::Error::other("no connection"));
        }
        if !self.stays_online.contains(path) {
            self.online.remove(path);
            self.free -= self.sizes[path];
        }
        Ok(())
    }
    fn is_online_only(&self, path: &Path) -> io::Result<bool> {
        Ok(self.online.contains(path))
    }
    fn size_on_laptop(&self, path: &Path) -> io::Result<u64> {
        Ok(self.sizes[path])
    }
    fn put_back(&mut self, path: &Path) -> io::Result<()> {
        self.log.push(format!("back {}", shown(path)));
        if self.refuse_put_back.contains(path) {
            return Err(io::Error::other("refused"));
        }
        if !self.slow_put_back.contains(path) {
            self.online.insert(path.to_path_buf());
            self.free += self.sizes[path];
        }
        Ok(())
    }
    fn free_space(&self) -> Option<u64> {
        Some(self.free)
    }
    fn marked_as_apps_own(&self, path: &Path) -> io::Result<bool> {
        Ok(self.apps_own.contains(path))
    }
    fn head(&self, path: &Path, n: usize) -> io::Result<Vec<u8>> {
        if self.fail_head.contains(path) {
            return Err(io::Error::other("unreadable"));
        }
        let mut c = self
            .content
            .get(path)
            .cloned()
            .unwrap_or_else(|| b"a photo".to_vec());
        c.truncate(n);
        Ok(c)
    }
}

fn consent() -> Consent {
    Consent {
        account_id: "me".into(),
        provider: CloudProvider::OneDrive,
        on_metered: false,
        home: home(),
        root: home().join("OneDrive"),
    }
}

fn wanted(owner: &str, files: &[(ItemId, PathBuf, u64)]) -> Vec<Wanted> {
    files
        .iter()
        .map(|(id, path, size)| Wanted {
            item: *id,
            account_id: owner.into(),
            provider: CloudProvider::OneDrive,
            path: path.clone(),
            size: *size,
        })
        .collect()
}

/// Batches never ask for more space than is free, keeping room to spare; a file that can never
/// fit stays safe in its cloud, said so.
#[test]
fn batches_fit_the_space_on_the_old_laptop() {
    let files = [
        (id(1), 3 * GB),
        (id(2), GB),
        (id(3), GB),
        (id(4), 20 * GB),
        (id(5), 2 * GB),
    ];
    let plan = plan_batches(&files, 5 * GB, 4 * GB);
    // 5 GB free, 1 GB kept free: at most 4 GB down at once, and never more than the batch size.
    for b in &plan.batches {
        assert!(b.bytes <= 4 * GB);
        assert_eq!(b.bytes, b.items.iter().map(|(_, s)| s).sum::<u64>());
    }
    let mut all: Vec<ItemId> = plan
        .batches
        .iter()
        .flat_map(|b: &Batch| b.items.iter().map(|(i, _)| *i))
        .collect();
    all.sort();
    assert_eq!(all, [id(1), id(2), id(3), id(5)]);
    assert_eq!(plan.too_big, [id(4)]);
    // In the order given, as many as fit in each batch.
    assert_eq!(plan.batches[0].items, [(id(1), 3 * GB), (id(2), GB)]);
    assert_eq!(plan.batches[1].items, [(id(3), GB), (id(5), 2 * GB)]);
    // Exactly the room there is fits; a byte more does not.
    let exact = plan_batches(&[(id(1), 4 * GB)], 5 * GB, 8 * GB);
    assert_eq!(exact.batches.len(), 1);
    assert!(exact.too_big.is_empty());
    let over = plan_batches(&[(id(1), 4 * GB + 1)], 5 * GB, 8 * GB);
    assert!(over.batches.is_empty());
    assert_eq!(over.too_big, [id(1)]);
    // No free space known below the room kept free: nothing comes down.
    let none = plan_batches(&[(id(1), 1)], KEEP_FREE_OLD, 8 * GB);
    assert!(none.batches.is_empty());
    assert_eq!(none.too_big, [id(1)]);
    // A batch size smaller than the room limits each batch.
    let small = plan_batches(&[(id(1), GB), (id(2), GB)], 10 * GB, GB);
    assert_eq!(small.batches.len(), 2);
}

fn id(n: u8) -> ItemId {
    online_only(person("me"), CloudProvider::OneDrive, &[&n.to_string()], 0).id
}

/// The whole round: down, checked, sent, back to online only, in that order, batch by batch.
#[test]
fn each_file_comes_down_is_sent_and_goes_back() {
    let mut fake = Fake {
        free: 10 * GB,
        ..Fake::default()
    };
    let a = fake.add("a.mp4", 2 * GB);
    let b = fake.add("b.mp4", 2 * GB);
    let mut keep = KeepLocal::new(os(), vec![consent()], 4 * GB);
    let mut sent = Vec::new();
    let report = keep.run(
        &mut fake,
        wanted("me", &[a.clone(), b.clone()]),
        &|| Network::Unmetered,
        &mut |batch: &[(ItemId, PathBuf)]| {
            sent.extend(batch.iter().map(|(i, _)| *i));
            batch.iter().map(|(i, _)| (*i, true)).collect()
        },
    );
    assert_eq!(report.stopped, None);
    assert_eq!(sent, [a.0, b.0]);
    for (i, _, _) in [&a, &b] {
        assert!(report.files[i].sent);
        assert_eq!(report.files[i].back_online_only, Some(true));
        assert_eq!(report.files[i].why_not, None);
    }
    assert_eq!(fake.online.len(), 2, "both are online only again");
    assert_eq!(fake.free, 10 * GB);
    assert_eq!(
        fake.log,
        ["down a.mp4", "down b.mp4", "back a.mp4", "back b.mp4"]
    );
    assert_eq!(report.words_for(&a.0), None);
    // The effect on the cloud app is recorded: what came down, and that it went back.
    assert_eq!(report.downloaded_bytes, 4 * GB);
    assert_eq!(report.still_on_old_laptop_bytes, 0);
}

/// No consent, no download: another person's files, or another provider, are left alone.
#[test]
fn nothing_comes_down_without_that_persons_consent() {
    let mut fake = Fake {
        free: 10 * GB,
        ..Fake::default()
    };
    let a = fake.add("a.txt", 10);
    let mut keep = KeepLocal::new(os(), vec![consent()], 4 * GB);
    let mut theirs = wanted("tunde", std::slice::from_ref(&a));
    let mut dropbox = wanted("me", std::slice::from_ref(&a));
    dropbox[0].provider = CloudProvider::Dropbox;
    dropbox[0].item = id(7);
    theirs.extend(dropbox);
    let report = keep.run(&mut fake, theirs, &|| Network::Unmetered, &mut |_| {
        panic!("nothing is sent")
    });
    assert!(fake.log.is_empty());
    assert_eq!(report.files[&a.0].why_not, Some(WhyNot::NoConsent));
    assert_eq!(
        report.words_for(&a.0).unwrap(),
        "Safe in OneDrive. It was not downloaded to keep a copy, because that was not chosen for it."
    );
    assert_eq!(report.files[&id(7)].why_not, Some(WhyNot::NoConsent));
}

/// T26-013: on a metered connection, nothing more comes down unless the person agreed to that
/// too; files already down are still sent and put back.
#[test]
fn a_metered_connection_stops_further_downloads() {
    let mut fake = Fake {
        free: 10 * GB,
        ..Fake::default()
    };
    let a = fake.add("a", GB);
    let b = fake.add("b", GB);
    let mut keep = KeepLocal::new(os(), vec![consent()], GB);
    let calls = std::cell::Cell::new(0);
    let report = keep.run(
        &mut fake,
        wanted("me", &[a.clone(), b.clone()]),
        &|| {
            calls.set(calls.get() + 1);
            if calls.get() == 1 {
                Network::Unmetered
            } else {
                Network::Metered
            }
        },
        &mut |batch| batch.iter().map(|(i, _)| (*i, true)).collect(),
    );
    assert_eq!(report.stopped, Some(Stopped::MeteredNow));
    assert!(report.files[&a.0].sent, "a batch of one is sent");
    assert_eq!(report.files[&a.0].back_online_only, Some(true));
    assert_eq!(report.files[&b.0].why_not, Some(WhyNot::Paused));
    assert_eq!(fake.log, ["down a", "back a"]);
    assert_eq!(
        report.stopped.unwrap().words(),
        "Paused: this connection may charge for data now. Keeping copies of online-only files continues on a free connection, or if you allow it."
    );
    // Not knowing whether it is metered counts as metered.
    let mut fake = Fake {
        free: 10 * GB,
        ..Fake::default()
    };
    let c = fake.add("c", GB);
    let report = keep.run(
        &mut fake,
        wanted("me", std::slice::from_ref(&c)),
        &|| Network::NotKnown,
        &mut |_| panic!(),
    );
    assert_eq!(report.stopped, Some(Stopped::MeteredNow));
    assert!(fake.log.is_empty());
    // With consent for metered connections, it goes on.
    let mut fake = Fake {
        free: 10 * GB,
        ..Fake::default()
    };
    let d = fake.add("d", GB);
    let mut ok = KeepLocal::new(
        os(),
        vec![Consent {
            on_metered: true,
            ..consent()
        }],
        GB,
    );
    let report = ok.run(
        &mut fake,
        wanted("me", std::slice::from_ref(&d)),
        &|| Network::Metered,
        &mut |b| b.iter().map(|(i, _)| (*i, true)).collect(),
    );
    assert_eq!(report.stopped, None);
    assert_eq!(report.files[&d.0].back_online_only, Some(true));
}

/// Space is checked again before every batch: a drive that filled up meanwhile stops the next
/// batch, with the files left safe in their cloud.
#[test]
fn a_full_drive_stops_before_the_next_batch() {
    let mut fake = Fake {
        free: 3 * GB,
        ..Fake::default()
    };
    let a = fake.add("a", GB);
    let b = fake.add("b", GB);
    // The cloud app does not put a back in time, so its gigabyte stays used.
    fake.slow_put_back.insert(at("a"));
    let mut keep = KeepLocal::new(os(), vec![consent()], GB);
    let report = keep.run(
        &mut fake,
        wanted("me", &[a.clone(), b.clone()]),
        &|| Network::Unmetered,
        &mut |batch| batch.iter().map(|(i, _)| (*i, true)).collect(),
    );
    // 2 GB free after a stays down, 1 GB kept free: b (1 GB) still fits.
    assert_eq!(report.files[&b.0].back_online_only, Some(true));
    // Now with only just enough for one.
    let mut fake = Fake {
        free: 2 * GB,
        ..Fake::default()
    };
    let a = fake.add("a", GB);
    let b = fake.add("b", GB);
    fake.slow_put_back.insert(at("a"));
    let report = keep.run(
        &mut fake,
        wanted("me", &[a.clone(), b.clone()]),
        &|| Network::Unmetered,
        &mut |batch| batch.iter().map(|(i, _)| (*i, true)).collect(),
    );
    assert_eq!(report.stopped, Some(Stopped::NoSpace));
    assert_eq!(report.files[&b.0].why_not, Some(WhyNot::Paused));
    assert_eq!(fake.log, ["down a", "back a"]);
    assert_eq!(
        Stopped::NoSpace.words(),
        "Paused: the old laptop is running out of space to download more files. The rest stay safe in their cloud."
    );
}

/// Every failure is its own honest outcome: a download that failed or did not really happen is
/// never sent; a file not sent is still put back; a file the cloud app did not put back is said
/// plainly, with the space it takes.
#[test]
fn every_failure_is_said_plainly() {
    let mut fake = Fake {
        free: 100 * GB,
        ..Fake::default()
    };
    let failed = fake.add("failed", GB);
    let fake_down = fake.add("fake_down", GB);
    let not_sent = fake.add("not_sent", GB);
    let refused = fake.add("refused", GB);
    let slow = fake.add("slow", GB);
    fake.fail_down.insert(failed.1.clone());
    fake.stays_online.insert(fake_down.1.clone());
    fake.refuse_put_back.insert(refused.1.clone());
    fake.slow_put_back.insert(slow.1.clone());
    let mut keep = KeepLocal::new(os(), vec![consent()], 100 * GB);
    let all = [
        failed.clone(),
        fake_down.clone(),
        not_sent.clone(),
        refused.clone(),
        slow.clone(),
    ];
    let mut offered = Vec::new();
    let report = keep.run(
        &mut fake,
        wanted("me", &all),
        &|| Network::Unmetered,
        &mut |batch| {
            offered.extend(batch.iter().map(|(_, p)| p.clone()));
            batch.iter().map(|(i, _)| (*i, *i != not_sent.0)).collect()
        },
    );
    assert_eq!(
        offered,
        [not_sent.1.clone(), refused.1.clone(), slow.1.clone()]
    );
    let f = &report.files;
    assert_eq!(f[&failed.0].why_not, Some(WhyNot::DownloadFailed));
    assert_eq!(f[&fake_down.0].why_not, Some(WhyNot::DownloadFailed));
    assert_eq!(f[&failed.0].back_online_only, None);
    assert!(!f[&not_sent.0].sent);
    assert_eq!(f[&not_sent.0].why_not, Some(WhyNot::NotSent));
    assert_eq!(f[&not_sent.0].back_online_only, Some(true));
    assert!(f[&refused.0].sent);
    assert_eq!(f[&refused.0].back_online_only, Some(false));
    assert_eq!(f[&slow.0].back_online_only, Some(false));
    assert_eq!(report.still_on_old_laptop_bytes, 2 * GB);
    assert_eq!(report.downloaded_bytes, 3 * GB);
    assert_eq!(
        report.words_for(&failed.0).unwrap(),
        "Safe in OneDrive. It could not be downloaded to keep a copy, so it stays online only."
    );
    assert_eq!(
        report.words_for(&slow.0).unwrap(),
        "Copied. OneDrive has not made it online only again on the old laptop, so it still takes space there. To free it, right-click it and choose Free up space."
    );
    assert_eq!(
        report.words_for(&not_sent.0).unwrap(),
        "Safe in OneDrive. It was downloaded but could not be sent, and it is online only again."
    );
    // Files never downloaded are never put back (nothing is asked of the cloud app for them).
    assert!(!fake.log.contains(&"back failed".to_string()));
    assert!(!fake.log.contains(&"back fake_down".to_string()));
}

/// A file that is already on the laptop is not part of this: it is copied as always, and the
/// cloud app is never asked to make it online only.
#[test]
fn a_file_already_on_the_laptop_is_never_put_back() {
    let mut fake = Fake {
        free: 10 * GB,
        ..Fake::default()
    };
    let a = fake.add("a", GB);
    fake.online.remove(&a.1);
    let mut keep = KeepLocal::new(os(), vec![consent()], GB);
    let report = keep.run(
        &mut fake,
        wanted("me", std::slice::from_ref(&a)),
        &|| Network::Unmetered,
        &mut |_| panic!("not sent here"),
    );
    assert!(fake.log.is_empty());
    assert_eq!(report.files[&a.0].why_not, Some(WhyNot::AlreadyOnLaptop));
}

/// A file whose size on the laptop after download differs from the plan is not sent as it.
#[test]
fn a_download_of_another_size_is_not_sent() {
    let mut fake = Fake {
        free: 10 * GB,
        ..Fake::default()
    };
    let a = fake.add("a", GB);
    let mut keep = KeepLocal::new(os(), vec![consent()], 2 * GB);
    let mut w = wanted("me", std::slice::from_ref(&a));
    w[0].size = GB + 1;
    let report = keep.run(&mut fake, w, &|| Network::Unmetered, &mut |_| {
        panic!("not sent")
    });
    assert_eq!(report.files[&a.0].why_not, Some(WhyNot::Changed));
    assert_eq!(report.files[&a.0].back_online_only, Some(true));
}

/// The Windows cloud app adapter on a real file that is not in a cloud folder: it reads, and
/// asking for online only cannot make an ordinary file online only, which is reported, not
/// hidden.
#[cfg(windows)]
#[test]
fn the_windows_adapter_on_an_ordinary_file() {
    use pctwin_orchestrate::local_copy::WindowsCloudFiles;
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("plain.txt");
    std::fs::write(&p, b"hello").unwrap();
    let system32 = std::path::Path::new(r"C:\Windows\System32");
    let mut w = WindowsCloudFiles::new(
        dir.path(),
        std::time::Duration::from_millis(300),
        Some(system32),
    );
    assert!(!w.is_online_only(&p).unwrap());
    w.bring_down(&p).unwrap();
    assert_eq!(w.size_on_laptop(&p).unwrap(), 5);
    // An ordinary file never becomes online only: asking is reported as not done.
    assert!(w.put_back(&p).is_err());
    assert!(!w.is_online_only(&p).unwrap());
    assert!(w.free_space().is_some());
    assert_eq!(std::fs::read(&p).unwrap(), b"hello");
}

/// A file bigger than the old laptop can ever hold stays safe in its cloud, said so; the rest
/// still come down.
#[test]
fn a_file_too_big_for_the_old_laptop_stays_in_its_cloud() {
    let mut fake = Fake {
        free: 5 * GB,
        ..Fake::default()
    };
    let huge = fake.add("huge", 20 * GB);
    let a = fake.add("a", GB);
    let mut keep = KeepLocal::new(os(), vec![consent()], 4 * GB);
    let report = keep.run(
        &mut fake,
        wanted("me", &[huge.clone(), a.clone()]),
        &|| Network::Unmetered,
        &mut |b| b.iter().map(|(i, _)| (*i, true)).collect(),
    );
    assert_eq!(report.stopped, None);
    assert_eq!(report.files[&huge.0].why_not, Some(WhyNot::TooBig));
    assert!(report.files[&a.0].sent);
    assert_eq!(
        report.words_for(&huge.0).unwrap(),
        "Safe in OneDrive. It is too big to download on the old laptop with the space it has."
    );
}

/// The move can only say it took a file of the batch it was given: a claim for any other file
/// changes nothing.
#[test]
fn a_send_claim_outside_the_batch_is_ignored() {
    let mut fake = Fake {
        free: 10 * GB,
        ..Fake::default()
    };
    let a = fake.add("a", GB);
    let c = fake.add("c", GB);
    let mut keep = KeepLocal::new(os(), vec![consent()], GB);
    let mut all = wanted("me", std::slice::from_ref(&a));
    all.extend(wanted("tunde", std::slice::from_ref(&c)));
    let report = keep.run(&mut fake, all, &|| Network::Unmetered, &mut |_batch| {
        vec![(a.0, true), (c.0, true)]
    });
    assert!(report.files[&a.0].sent);
    assert!(!report.files[&c.0].sent, "not in the batch, so not taken");
    assert_eq!(report.files[&c.0].why_not, Some(WhyNot::NoConsent));
}

/// Free space is the volume mounted deepest above the cloud folder.
#[test]
fn free_space_comes_from_the_deepest_volume() {
    use pctwin_orchestrate::local_copy::free_space_of;
    use pctwin_scan::{DiskKind, Volume, VolumeKind};
    let vol = |mount: &str, free: u64| Volume {
        mount: PathBuf::from(mount),
        name: String::new(),
        kind: VolumeKind::Internal,
        file_system: "NTFS".into(),
        total_bytes: 100,
        free_bytes: free,
        disk: DiskKind::Ssd,
        is_system: false,
        skip: Vec::new(),
        also: Vec::new(),
    };
    let vols = [vol("/", 1), vol("/mnt/data", 2), vol("/mnt", 3)];
    assert_eq!(
        free_space_of(Path::new("/mnt/data/OneDrive"), &vols),
        Some(2)
    );
    assert_eq!(free_space_of(Path::new("/home/a"), &vols), Some(1));
    assert_eq!(free_space_of(Path::new("/home/a"), &vols[1..]), None);
}

/// Third review, finding 4: a sync app's sign-in tokens and the system's credential stores are
/// never brought down or sent, whatever they are called, even with consent.
#[test]
fn sign_in_tokens_are_never_in_scope() {
    let mut fake = Fake {
        free: 100 * GB,
        ..Fake::default()
    };
    let tokens: &[&str] = if cfg!(windows) {
        &[
            r"C:\Users\me\AppData\Local\Microsoft\OneDrive\settings\Personal\a.dat",
            r"C:\Users\me\OneDrive\..\AppData\Local\Microsoft\Credentials\x",
            r"C:\Users\me\ONEDRI~1\x",
            r"C:\Users\me\OneDrive.\x",
            r"C:\Users\other\OneDrive\x",
            r"D:\OneDrive\x",
        ]
    } else {
        &[
            "/home/me/.config/onedrive/refresh_token",
            "/home/me/OneDrive/../.config/onedrive/refresh_token",
            "/home/me/onedrive/x",
            "/home/other/OneDrive/x",
        ]
    };
    let mut files: Vec<_> = tokens.iter().map(|p| fake.add(p, 1)).collect();
    // Inside the synced folder: the app's own hidden folder, a keychain, a cookie database.
    for p in [
        ".849C9593-D756-4E56/settings.dat",
        "login.keychain-db",
        "Cookies",
    ] {
        files.push(fake.add(p, 1));
    }
    // Marked by the system as the app's own.
    let marked = fake.add("desktop-sync-state.bin", 1);
    fake.apps_own.insert(marked.1.clone());
    files.push(marked);
    let mut keep = KeepLocal::new(os(), vec![consent()], 4 * GB);
    let mut sent = 0;
    let report = keep.run(
        &mut fake,
        wanted("me", &files),
        &|| Network::Unmetered,
        &mut |b| {
            sent += b.len();
            b.iter().map(|(i, _)| (*i, true)).collect()
        },
    );
    assert_eq!(sent, 0, "a token was sent");
    assert!(fake.log.is_empty(), "{:?}", fake.log);
    for (id, _, _) in &files {
        assert_eq!(report.files[id].why_not, Some(WhyNot::OutOfScope));
    }
    assert_eq!(
        report.words_for(&files[0].0).unwrap(),
        "Not copied. PCTwin keeps copies only of the files you keep in OneDrive, never the app's own sign-in or settings. Sign in to OneDrive on the new laptop."
    );
    // The person's own files in that folder are in scope.
    let own = fake.add("Documents/thesis.docx", 1);
    let report = keep.run(
        &mut fake,
        wanted("me", std::slice::from_ref(&own)),
        &|| Network::Unmetered,
        &mut |b| b.iter().map(|(i, _)| (*i, true)).collect(),
    );
    assert!(report.files[&own.0].sent);
}

/// What a file holds is checked once it is down: token text or a data-protection blob is never
/// sent, and is put back like any other.
#[test]
fn a_file_holding_tokens_is_never_sent() {
    let mut fake = Fake {
        free: 100 * GB,
        ..Fake::default()
    };
    let notes = fake.add("Apps/sync-notes.json", 20);
    fake.content
        .insert(notes.1.clone(), br#"{"refresh_token":"abc"}"#.to_vec());
    let mut keep = KeepLocal::new(os(), vec![consent()], 4 * GB);
    let report = keep.run(
        &mut fake,
        wanted("me", std::slice::from_ref(&notes)),
        &|| Network::Unmetered,
        &mut |_| panic!("never sent"),
    );
    assert_eq!(report.files[&notes.0].why_not, Some(WhyNot::OutOfScope));
    assert_eq!(report.files[&notes.0].back_online_only, Some(true));
    // What cannot be read cannot be told apart from account state: not sent either.
    let blind = fake.add("Apps/unreadable.bin", 20);
    fake.fail_head.insert(blind.1.clone());
    let report = keep.run(
        &mut fake,
        wanted("me", std::slice::from_ref(&blind)),
        &|| Network::Unmetered,
        &mut |_| panic!("never sent"),
    );
    assert_eq!(report.files[&blind.0].why_not, Some(WhyNot::OutOfScope));
}

/// A sync app PCTwin has never heard of works the same: its folder, found on the laptop, is in
/// scope; its own hidden files are not.
#[test]
fn an_unknown_sync_apps_folder_is_handled_the_same() {
    let mut fake = Fake {
        free: 100 * GB,
        ..Fake::default()
    };
    let root = home().join("SomeSync");
    let mine = fake.add(&root.join("photo.jpg").to_string_lossy(), 1);
    let state = fake.add(&root.join(".somesync").join("db").to_string_lossy(), 1);
    let consent = Consent {
        account_id: "me".into(),
        provider: CloudProvider::Other,
        on_metered: false,
        home: home(),
        root,
    };
    let mut w = wanted("me", &[mine.clone(), state.clone()]);
    for x in &mut w {
        x.provider = CloudProvider::Other;
    }
    let mut keep = KeepLocal::new(os(), vec![consent], 4 * GB);
    let report = keep.run(&mut fake, w, &|| Network::Unmetered, &mut |b| {
        b.iter().map(|(i, _)| (*i, true)).collect()
    });
    assert!(report.files[&mine.0].sent);
    assert_eq!(report.files[&state.0].why_not, Some(WhyNot::OutOfScope));
}

/// Third review, finding 7: the Windows adapter touches only files inside the consented
/// person's cloud folder, never one outside it or reached through a link.
#[cfg(windows)]
#[test]
fn the_windows_adapter_stays_in_the_consented_folder() {
    use pctwin_orchestrate::local_copy::WindowsCloudFiles;
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("OneDrive");
    let outside = dir.path().join("Other");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    let theirs = outside.join("secret.txt");
    std::fs::write(&theirs, b"theirs").unwrap();
    let ok = std::process::Command::new("cmd")
        .args(["/d", "/c", "mklink", "/J"])
        .arg(root.join("link"))
        .arg(&outside)
        .output()
        .unwrap();
    assert!(ok.status.success(), "{ok:?}");
    let system32 = std::path::Path::new(r"C:\Windows\System32");
    let mut w =
        WindowsCloudFiles::new(&root, std::time::Duration::from_millis(100), Some(system32));
    assert!(w.bring_down(&theirs).is_err(), "outside the folder");
    // Refused before attrib is ever asked: not a wait that ran out.
    assert_eq!(
        w.put_back(&theirs).unwrap_err().kind(),
        std::io::ErrorKind::PermissionDenied
    );
    let through = root.join("link").join("secret.txt");
    assert!(w.bring_down(&through).is_err(), "through a link");
    assert_eq!(
        w.put_back(&through).unwrap_err().kind(),
        std::io::ErrorKind::PermissionDenied
    );
    let dotdot = root.join("..").join("Other").join("secret.txt");
    assert!(w.bring_down(&dotdot).is_err());
    // A file of its own is touched, and attrib runs from the system folder given, never one an
    // environment value names: with no system folder known, nothing is put back.
    let mine = root.join("mine.txt");
    std::fs::write(&mine, b"mine").unwrap();
    assert!(w.bring_down(&mine).is_ok());
    let mut none = WindowsCloudFiles::new(&root, std::time::Duration::from_millis(100), None);
    let e = none.put_back(&mine).unwrap_err();
    assert!(e.to_string().contains("system folder"), "{e}");
    let fake = tempfile::tempdir().unwrap();
    let mut elsewhere = WindowsCloudFiles::new(
        &root,
        std::time::Duration::from_millis(100),
        Some(fake.path()),
    );
    assert!(
        elsewhere.put_back(&mine).is_err(),
        "no attrib in that folder"
    );
}

/// The cloud app is asked with Windows' own attrib, from the system folder given, started there.
#[test]
fn attrib_is_the_systems_own_started_in_its_folder() {
    use pctwin_orchestrate::local_copy::attrib_command;
    let system32 = Path::new(if cfg!(windows) {
        r"C:\Windows\System32"
    } else {
        "/sys32"
    });
    let c = attrib_command(system32, Path::new("x.txt"));
    assert_eq!(c.get_program(), system32.join("attrib.exe").as_os_str());
    assert_eq!(c.get_current_dir(), Some(system32));
    let args: Vec<_> = c.get_args().collect();
    assert_eq!(args, ["-P", "+U", "x.txt"]);
}

/// An app that syncs the whole home folder: the known places of account state inside it are
/// still never taken.
#[test]
fn a_whole_home_sync_still_leaves_account_state() {
    let mut fake = Fake {
        free: 100 * GB,
        ..Fake::default()
    };
    // A known place of this system's account state (the shipped catalogue's).
    let place: &[&str] = if cfg!(windows) {
        &["AppData", "Local", "Microsoft", "Credentials"]
    } else if cfg!(target_os = "macos") {
        &["Library", "Keychains"]
    } else {
        &[".local", "share", "keyrings"]
    };
    let state = place.iter().fold(home(), |p, n| p.join(n)).join("x");
    let state = fake.add(&state.to_string_lossy(), 1);
    let mut consent = consent();
    consent.root = home();
    let mut keep = KeepLocal::new(os(), vec![consent], 4 * GB);
    let which = &state;
    let report = keep.run(
        &mut fake,
        wanted("me", std::slice::from_ref(which)),
        &|| Network::Unmetered,
        &mut |_| panic!("never sent"),
    );
    assert_eq!(report.files[&which.0].why_not, Some(WhyNot::OutOfScope));
}
