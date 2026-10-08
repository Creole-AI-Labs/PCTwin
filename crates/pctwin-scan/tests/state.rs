//! Saving the scan (Task List 1.4): progress is saved after each folder so a restart resumes where
//! it stopped; a later scan says exactly what was added, removed or changed; a damaged or newer
//! saved scan is refused safely; saving never leaves a half-written file.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use pctwin_record::{FolderRole, LaptopId, Storage};
use pctwin_scan::{
    Flow, FolderLookup, FoundFolder, Person, ScanState, StateError, build_scan_resumable,
    changes_between,
};

fn laptop() -> LaptopId {
    LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap()
}

fn me(home: &Path) -> Vec<Person> {
    vec![Person {
        account_id: "1000".into(),
        suggested_name: "ada".into(),
        home: home.to_path_buf(),
        is_me: true,
    }]
}

fn found(role: FolderRole, path: &Path) -> FolderLookup {
    FolderLookup::Found(FoundFolder {
        role,
        path: path.to_path_buf(),
        storage: Storage::SystemDrive,
        moved: false,
    })
}

fn write(path: PathBuf, bytes: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

fn age(path: &Path, seconds: u64) {
    let file = std::fs::File::options().write(true).open(path).unwrap();
    file.set_modified(SystemTime::now() - Duration::from_secs(seconds))
        .unwrap();
}

fn two_folders(base: &Path) -> Vec<FolderLookup> {
    write(base.join("docs/a.txt"), b"a");
    write(base.join("pics/b.jpg"), b"b");
    vec![
        found(FolderRole::Documents, &base.join("docs")),
        found(FolderRole::Pictures, &base.join("pics")),
    ]
}

fn item_names(state: &ScanState) -> Vec<String> {
    let mut v: Vec<String> = state
        .record()
        .items
        .iter()
        .map(|i| {
            i.path
                .parts()
                .iter()
                .map(|n| n.display())
                .collect::<Vec<_>>()
                .join("/")
        })
        .collect();
    v.sort();
    v
}

#[test]
fn a_saved_scan_loads_back_exactly() {
    let base = tempfile::tempdir().unwrap();
    let folders = two_folders(base.path());
    let scan = build_scan_resumable(
        laptop(),
        me(base.path()),
        folders,
        Vec::new(),
        None,
        &mut |_| Flow::Continue,
    )
    .unwrap();
    let file = base.path().join("scan.json");
    scan.state.save(&file).unwrap();
    let back = ScanState::load(&file).unwrap();
    assert_eq!(back, scan.state);
    assert!(back.is_finished());
}

#[test]
fn saving_replaces_the_old_save_and_leaves_nothing_behind() {
    let base = tempfile::tempdir().unwrap();
    let saves = tempfile::tempdir().unwrap();
    let file = saves.path().join("scan.json");
    std::fs::write(&file, b"old").unwrap();
    let folders = two_folders(base.path());
    let scan = build_scan_resumable(
        laptop(),
        me(base.path()),
        folders,
        Vec::new(),
        None,
        &mut |_| Flow::Continue,
    )
    .unwrap();
    scan.state.save(&file).unwrap();
    scan.state.save(&file).unwrap();
    assert_eq!(ScanState::load(&file).unwrap(), scan.state);
    let left: Vec<_> = std::fs::read_dir(saves.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(left, ["scan.json"], "temporary files left behind");
}

#[test]
fn a_damaged_or_newer_save_is_refused_safely() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("scan.json");
    std::fs::write(&file, b"{ not json").unwrap();
    assert!(matches!(
        ScanState::load(&file),
        Err(StateError::Damaged(_))
    ));
    std::fs::write(&file, br#"{"format": 99, "anything": true}"#).unwrap();
    assert!(matches!(
        ScanState::load(&file),
        Err(StateError::NewerFormat { found: 99 })
    ));
    assert!(matches!(
        ScanState::load(&dir.path().join("missing.json")),
        Err(StateError::Io(_))
    ));
}

#[test]
fn a_restart_resumes_after_the_last_finished_folder() {
    let base = tempfile::tempdir().unwrap();
    let folders = two_folders(base.path());
    // Stop (as if the laptop restarted) right after the first folder is saved.
    let mut saved = None;
    let stopped = build_scan_resumable(
        laptop(),
        me(base.path()),
        folders.clone(),
        Vec::new(),
        None,
        &mut |state| {
            saved = Some(state.clone());
            Flow::Stop
        },
    )
    .unwrap();
    assert!(!stopped.state.is_finished());
    let saved = saved.unwrap();
    assert_eq!(saved.done_folders().len(), 1);

    // The finished folder is not scanned again: even if it changed since, its saved items are used.
    let done = saved.done_folders()[0].clone();
    std::fs::remove_dir_all(&done).unwrap();
    let resumed = build_scan_resumable(
        laptop(),
        me(base.path()),
        folders,
        Vec::new(),
        Some(&saved),
        &mut |_| Flow::Continue,
    )
    .unwrap();
    assert!(resumed.state.is_finished());
    assert_eq!(item_names(&resumed.state), ["a.txt", "b.jpg"]);
    // Scanned once each: the removed folder was not looked at again.
    assert_eq!(resumed.state.done_folders().len(), 2);
    assert!(resumed.unreadable.is_empty(), "{:?}", resumed.unreadable);
    resumed.state.record().validate().unwrap();
}

#[test]
fn a_save_from_another_person_or_laptop_is_not_resumed() {
    let base = tempfile::tempdir().unwrap();
    let folders = two_folders(base.path());
    let mut saved = None;
    build_scan_resumable(
        laptop(),
        me(base.path()),
        folders.clone(),
        Vec::new(),
        None,
        &mut |state| {
            saved = Some(state.clone());
            Flow::Stop
        },
    )
    .unwrap();
    let saved = saved.unwrap();
    let other_laptop = LaptopId::from_hex("ffeeddccbbaa99887766554433221100").unwrap();
    let fresh = build_scan_resumable(
        other_laptop,
        me(base.path()),
        folders,
        Vec::new(),
        Some(&saved),
        &mut |_| Flow::Continue,
    )
    .unwrap();
    assert_eq!(fresh.state.done_folders().len(), 2);
    assert!(
        fresh
            .state
            .record()
            .items
            .iter()
            .all(|i| i.id != saved.record().items[0].id)
    );
}

#[test]
fn a_later_scan_says_what_was_added_removed_or_changed() {
    let base = tempfile::tempdir().unwrap();
    write(base.path().join("docs/same.txt"), b"same");
    write(base.path().join("docs/grows.txt"), b"1");
    write(base.path().join("docs/goes.txt"), b"x");
    for f in ["same.txt", "grows.txt", "goes.txt"] {
        age(&base.path().join("docs").join(f), 3600);
    }
    let folders = vec![found(FolderRole::Documents, &base.path().join("docs"))];
    let scan = |folders: Vec<FolderLookup>| {
        build_scan_resumable(
            laptop(),
            me(base.path()),
            folders,
            Vec::new(),
            None,
            &mut |_| Flow::Continue,
        )
        .unwrap()
        .state
    };
    let before = scan(folders.clone());

    write(base.path().join("docs/grows.txt"), b"1234");
    age(&base.path().join("docs/grows.txt"), 3600);
    std::fs::remove_file(base.path().join("docs/goes.txt")).unwrap();
    write(base.path().join("docs/new.txt"), b"n");
    let after = scan(folders);

    let c = changes_between(&before, &after);
    let names = |ids: &[pctwin_record::ItemId], state: &ScanState| -> Vec<String> {
        let mut v: Vec<String> = ids
            .iter()
            .map(|id| {
                let item = state.record().items.iter().find(|i| i.id == *id).unwrap();
                item.path.parts()[0].display().to_string()
            })
            .collect();
        v.sort();
        v
    };
    assert_eq!(names(&c.added, &after), ["new.txt"]);
    assert_eq!(names(&c.removed, &before), ["goes.txt"]);
    assert_eq!(names(&c.changed, &after), ["grows.txt"]);
    assert_eq!(names(&c.unchanged, &after), ["same.txt"]);
}

#[test]
fn a_file_touched_right_around_the_scan_counts_as_changed() {
    let base = tempfile::tempdir().unwrap();
    write(base.path().join("docs/fresh.txt"), b"same");
    let folders = vec![found(FolderRole::Documents, &base.path().join("docs"))];
    let scan = || {
        build_scan_resumable(
            laptop(),
            me(base.path()),
            folders.clone(),
            Vec::new(),
            None,
            &mut |_| Flow::Continue,
        )
        .unwrap()
        .state
    };
    // Written moments before the first scan: an edit in the same clock tick could look identical,
    // so it is checked again rather than trusted (the "racy git" rule).
    let before = scan();
    let after = scan();
    let c = changes_between(&before, &after);
    assert_eq!(c.changed.len(), 1);
    assert!(c.unchanged.is_empty());
}

#[test]
fn a_failed_save_leaves_the_old_save_and_no_temporary_file() {
    let base = tempfile::tempdir().unwrap();
    let saves = tempfile::tempdir().unwrap();
    let folders = two_folders(base.path());
    let scan = build_scan_resumable(
        laptop(),
        me(base.path()),
        folders,
        Vec::new(),
        None,
        &mut |_| Flow::Continue,
    )
    .unwrap();
    // A folder sits where the save should go, so moving the new save into place fails.
    let target = saves.path().join("scan.json");
    std::fs::create_dir(&target).unwrap();
    std::fs::write(target.join("keep.txt"), b"keep").unwrap();
    assert!(scan.state.save(&target).is_err());
    let left: Vec<_> = std::fs::read_dir(saves.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(left, ["scan.json"]);
    assert!(target.join("keep.txt").exists());
}

#[test]
fn a_save_whose_record_breaks_the_rules_is_refused() {
    let base = tempfile::tempdir().unwrap();
    let folders = two_folders(base.path());
    let scan = build_scan_resumable(
        laptop(),
        me(base.path()),
        folders,
        Vec::new(),
        None,
        &mut |_| Flow::Continue,
    )
    .unwrap();
    let file = base.path().join("scan.json");
    scan.state.save(&file).unwrap();
    // Tamper with the record inside: the same item twice.
    let mut saved: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
    let mut record: serde_json::Value =
        serde_json::from_str(saved["record"].as_str().unwrap()).unwrap();
    let first = record["items"][0].clone();
    record["items"].as_array_mut().unwrap().push(first);
    saved["record"] = serde_json::Value::String(record.to_string());
    std::fs::write(&file, serde_json::to_vec(&saved).unwrap()).unwrap();
    assert!(matches!(
        ScanState::load(&file),
        Err(StateError::Damaged(_))
    ));
}
