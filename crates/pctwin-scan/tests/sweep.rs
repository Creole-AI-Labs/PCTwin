//! The drive-wide sweep (Engineering Plan 9.16; Product Spec section 6, "Everywhere, not just
//! the usual folders" and "Every place accounted for"), on small laptops built in a temporary
//! folder: two "drives", people's homes, things planted in odd places.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use pctwin_record::{
    FolderRole, Inclusion, Item, LaptopId, LeftOutReason, ManagedBy, Owner, Storage,
};
use pctwin_scan::{
    AccessFact, AccessReader, BlindReason, Census, DiskKind, Fate, FolderLookup, FoundBy,
    FoundFolder, PersonKind, Resident, Rule, Rules, Sweep, SweepFacts, SweepLimits, SystemKind,
    TopLine, UnixMode, Volume, VolumeKind, Whose, access_from_sddl, blind_reason_for, sweep,
};

const ADA: &str = "S-1-5-21-111-222-333-1001";
const BEA: &str = "S-1-5-21-111-222-333-1002";

fn laptop() -> LaptopId {
    LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap()
}

fn write(path: &Path, bytes: usize) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, vec![b'x'; bytes]).unwrap();
}

/// Access answers by folder name; everything else is readable by everyone.
#[derive(Default)]
struct Answers(BTreeMap<String, AccessFact>);

impl AccessReader for Answers {
    fn read(&mut self, path: &Path, _unix: Option<UnixMode>) -> AccessFact {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        self.0
            .get(&name)
            .cloned()
            .unwrap_or_else(|| AccessFact::Known(access_from_sddl("O:BAD:(A;;FR;;;WD)").unwrap()))
    }
}

struct Lab {
    _dir: tempfile::TempDir,
    c: PathBuf,
    d: PathBuf,
    ada: PathBuf,
    bea: PathBuf,
}

/// Two drives: C (the system's) with Users\ada and Users\bea, and D.
fn lab() -> Lab {
    let dir = tempfile::tempdir().unwrap();
    let c = dir.path().join("C");
    let d = dir.path().join("D");
    let ada = c.join("Users").join("ada");
    let bea = c.join("Users").join("bea");
    for p in [&c, &d, &ada, &bea] {
        std::fs::create_dir_all(p).unwrap();
    }
    Lab {
        _dir: dir,
        c,
        d,
        ada,
        bea,
    }
}

fn resident(id: &str, kind: PersonKind, home: &Path) -> Resident {
    Resident {
        account_id: Some(id.into()),
        kind,
        suggested_name: id.into(),
        home: Some(home.to_path_buf()),
        found_by: vec![FoundBy::AccountList],
    }
}

fn census(lab: &Lab, bea: bool) -> Census {
    let mut residents = vec![resident(ADA, PersonKind::Me, &lab.ada)];
    if bea {
        residents.push(resident(BEA, PersonKind::Other, &lab.bea));
    }
    Census {
        me: ADA.into(),
        profiles_root: Some(lab.c.join("Users")),
        residents,
        account_list_read: true,
        system: SystemKind::this(),
        uid_range: (1000, 60000),
    }
}

fn volume(mount: &Path, system: bool, kind: VolumeKind, disk: DiskKind) -> Volume {
    Volume {
        mount: mount.to_path_buf(),
        name: String::new(),
        kind,
        file_system: "NTFS".into(),
        total_bytes: 0,
        free_bytes: 0,
        disk,
        is_system: system,
        skip: Vec::new(),
        also: Vec::new(),
    }
}

fn known(lab: &Lab) -> Vec<FolderLookup> {
    vec![FolderLookup::Found(FoundFolder {
        role: FolderRole::Documents,
        path: lab.ada.join("Documents"),
        storage: Storage::SystemDrive,
        moved: false,
        managed_by: ManagedBy::Personal,
    })]
}

fn facts(lab: &Lab, several: bool) -> SweepFacts {
    SweepFacts {
        laptop: laptop(),
        census: census(lab, several),
        volumes: vec![
            volume(&lab.c, true, VolumeKind::Internal, DiskKind::Ssd),
            volume(&lab.d, false, VolumeKind::Internal, DiskKind::Hdd),
        ],
        known_folders: known(lab),
        remembered: Vec::new(),
        rules: Rules::shipped(),
    }
}

fn run(facts: SweepFacts, answers: &mut Answers) -> Sweep {
    let stop = AtomicBool::new(false);
    sweep(
        facts,
        answers,
        &stop,
        &mut |_| {},
        &SweepLimits::default(),
        None,
    )
}

fn shown(item: &Item) -> String {
    item.path
        .parts()
        .iter()
        .map(|n| n.display())
        .collect::<Vec<_>>()
        .join("/")
}

fn find<'a>(s: &'a Sweep, role: FolderRole, path: &str) -> &'a Item {
    s.record
        .items
        .iter()
        .find(|i| i.place.role == role && shown(i) == path)
        .unwrap_or_else(|| {
            panic!(
                "{path} not found in {:#?}",
                s.record.items.iter().map(shown).collect::<Vec<_>>()
            )
        })
}

fn mover() -> Owner {
    Owner::Person {
        account_id: ADA.into(),
    }
}

/// Sets each volume's size so that it is exactly as full as what was planted plus `extra`.
fn sized(mut f: SweepFacts, extra: u64) -> SweepFacts {
    for v in &mut f.volumes {
        let used = du(&v.mount);
        v.total_bytes = 1 << 40;
        v.free_bytes = v.total_bytes - used - extra;
    }
    f
}

fn du(path: &Path) -> u64 {
    let mut total = 0;
    let mut pending = vec![path.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in rd.flatten() {
            let Ok(t) = e.file_type() else { continue };
            if t.is_dir() {
                pending.push(e.path());
            } else if t.is_file() {
                total += e.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }
    }
    total
}

#[test]
fn things_in_odd_places_on_every_drive_are_found_and_the_systems_are_only_counted() {
    let lab = lab();
    write(&lab.c.join("Projects/app/main.rs"), 100);
    write(&lab.d.join("Photos/2019/beach.jpg"), 200);
    write(&lab.d.join("notes.txt"), 10);
    // Each system's own shapes: its system folder, its programs, a program's data in the home.
    let (system_dir, apps_dir, app_data, junk) = if cfg!(windows) {
        (
            "Windows",
            "Program Files",
            "AppData/Local/Google",
            "AppData/Local/Temp",
        )
    } else if cfg!(target_os = "macos") {
        (
            "System",
            "Applications",
            "Library/Application Support/Google",
            "Library/Caches",
        )
    } else {
        ("usr", "opt", ".config/Google", ".cache")
    };
    write(&lab.c.join(system_dir).join("System32/kernel.dll"), 5000);
    write(&lab.c.join(apps_dir).join("App/app.exe"), 3000);
    write(&lab.ada.join("Documents/cv.docx"), 40);
    write(&lab.ada.join("code/site/index.html"), 50);
    write(
        &lab.ada.join("code/site/node_modules/left-pad/index.js"),
        70,
    );
    write(
        &lab.ada.join("code/site/node_modules/left-pad/package.json"),
        30,
    );
    write(&lab.ada.join(app_data).join("Chrome/prefs"), 60);
    write(&lab.ada.join(junk).join("junk.tmp"), 15);
    let s = run(sized(facts(&lab, false), 0), &mut Answers::default());

    assert!(s.finished);
    assert_eq!(
        find(&s, FolderRole::Other, "Projects/app/main.rs").owner,
        mover()
    );
    let beach = find(&s, FolderRole::Other, "Photos/2019/beach.jpg");
    assert_eq!(beach.owner, mover());
    assert_eq!(
        beach.place.storage,
        Storage::OtherDrive {
            drive: lab
                .d
                .to_string_lossy()
                .trim_end_matches(['\\', '/'])
                .to_string()
        }
    );
    find(&s, FolderRole::Other, "notes.txt");
    assert_eq!(find(&s, FolderRole::Documents, "cv.docx").owner, mover());
    find(&s, FolderRole::Home, "code/site/index.html");
    // node_modules is one item, with its whole size.
    let nm = find(&s, FolderRole::Home, "code/site/node_modules");
    assert_eq!(nm.size_bytes, 100);
    assert!(!s.record.items.iter().any(|i| shown(i).contains("left-pad")));
    // The system's and the apps' are counted, never listed; app data is listed by program as
    // "found, not moved in this version".
    assert!(!s.record.items.iter().any(|i| shown(i).contains("kernel")));
    assert!(!s.record.items.iter().any(|i| shown(i).contains("prefs")));
    let chrome = s
        .places
        .iter()
        .find(|p| p.path.as_deref().is_some_and(|x| x.ends_with("Google")))
        .expect("app data counted per program");
    assert_eq!(chrome.fate, Fate::FoundNotMovedInThisVersion);
    assert_eq!(chrome.bytes, 60);
    let windows = s
        .places
        .iter()
        .find(|p| p.path.as_deref().is_some_and(|x| x.ends_with(system_dir)))
        .unwrap();
    assert_eq!(windows.placement.whose, Whose::System);
    assert!(matches!(&windows.placement.rule, Rule::RulesFile { row } if row.contains(system_dir)));
    assert_eq!(windows.bytes, 5000);
    // Every item says what placed it.
    for item in &s.record.items {
        assert!(s.placement_of(&item.id).is_some(), "{}", shown(item));
    }
    assert_eq!(
        s.placement_of(&find(&s, FolderRole::Other, "Projects/app/main.rs").id)
            .unwrap()
            .rule,
        Rule::OnePersonLaptop
    );
    // Nothing unexplained: every byte the drives say is used was seen.
    for v in &s.volumes {
        assert_eq!(v.unexplained_bytes, 0, "{v:?}");
        assert!(v.walked);
    }
    assert!(s.blind_spots.is_empty(), "{:?}", s.blind_spots);
    assert_eq!(s.top_line, TopLine::Everything, "{:#?}", s.blind_spots);
}

#[test]
fn one_walker_on_a_hard_disk_and_up_to_four_on_an_ssd() {
    assert_eq!(DiskKind::Hdd.walkers(4), 1);
    assert_eq!(DiskKind::Unknown.walkers(4), 1);
    assert_eq!(DiskKind::Ssd.walkers(4), 4);
    assert_eq!(DiskKind::Ssd.walkers(16), 4);
    let lab = lab();
    write(&lab.d.join("x/y.txt"), 1);
    let s = run(sized(facts(&lab, false), 0), &mut Answers::default());
    assert_eq!(s.volumes[0].walkers, 4);
    assert_eq!(s.volumes[1].walkers, 1);
}

#[test]
fn photos_on_d_made_by_one_person_but_readable_by_users_are_shared() {
    let lab = lab();
    write(&lab.d.join("Photos/beach.jpg"), 200);
    write(&lab.d.join("AdaOnly/diary.txt"), 20);
    let mut answers = Answers::default();
    answers.0.insert(
        "Photos".into(),
        AccessFact::Known(
            access_from_sddl(&format!(
                "O:{BEA}D:AI(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;OICI;0x1200a9;;;BU)(A;OICIID;FA;;;{BEA})"
            ))
            .unwrap(),
        ),
    );
    answers.0.insert(
        "AdaOnly".into(),
        AccessFact::Known(
            access_from_sddl(&format!("O:BAD:P(A;OICI;FA;;;BA)(A;OICI;FA;;;{ADA})")).unwrap(),
        ),
    );
    let s = run(sized(facts(&lab, true), 0), &mut answers);
    let beach = find(&s, FolderRole::Other, "Photos/beach.jpg");
    assert_eq!(beach.owner, Owner::Shared);
    let rule = &s.placement_of(&beach.id).unwrap().rule;
    assert_eq!(*rule, Rule::EveryoneCanRead);
    assert_eq!(
        find(&s, FolderRole::Other, "AdaOnly/diary.txt").owner,
        mover()
    );
}

#[test]
fn a_narrower_folder_inside_a_shared_one_is_not_shared() {
    let lab = lab();
    write(&lab.d.join("Family/holiday.jpg"), 10);
    write(&lab.d.join("Family/BeaPrivate/letter.txt"), 10);
    let mut answers = Answers::default();
    answers.0.insert(
        "BeaPrivate".into(),
        AccessFact::Known(
            access_from_sddl(&format!("O:{BEA}D:P(A;;FA;;;BA)(A;OICI;FA;;;{BEA})")).unwrap(),
        ),
    );
    let s = run(sized(facts(&lab, true), 0), &mut answers);
    assert_eq!(
        find(&s, FolderRole::Other, "Family/holiday.jpg").owner,
        Owner::Shared
    );
    // Bea's own folder is not listed for Ada at all.
    assert!(!s.record.items.iter().any(|i| shown(i).contains("letter")));
    assert!(
        !s.record
            .items
            .iter()
            .any(|i| shown(i).contains("BeaPrivate"))
    );
    let bea = s
        .places
        .iter()
        .find(|p| {
            p.placement.whose
                == Whose::Person {
                    account_id: BEA.into(),
                }
        })
        .unwrap();
    assert_eq!(bea.path, None);
    assert_eq!(bea.fate, Fate::NotYours);
}

/// Makes `dir` unreadable to this account; returns a guard that undoes it.
struct Locked(PathBuf);

impl Locked {
    fn new(dir: &Path) -> Self {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o000)).unwrap();
        }
        #[cfg(windows)]
        {
            let me = whoami();
            let ok = std::process::Command::new("icacls")
                .arg(dir)
                .args(["/deny", &format!("*{me}:(OI)(CI)(RD)")])
                .output()
                .unwrap()
                .status
                .success();
            assert!(ok);
        }
        Self(dir.to_path_buf())
    }
}

impl Drop for Locked {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o755));
        }
        #[cfg(windows)]
        {
            let _ = std::process::Command::new("icacls")
                .arg(&self.0)
                .args(["/remove:d", &format!("*{}", whoami())])
                .output();
            // icacls took the denied right out of the allowed ones too: give it back.
            let _ = std::process::Command::new("icacls")
                .arg(&self.0)
                .args(["/grant", &format!("*{}:(OI)(CI)F", whoami())])
                .output();
        }
    }
}

#[cfg(windows)]
fn whoami() -> String {
    let out = std::process::Command::new("whoami")
        .args(["/user", "/fo", "csv", "/nh"])
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    text.trim()
        .rsplit(',')
        .next()
        .unwrap()
        .trim_matches('"')
        .to_string()
}

/// Whether this account can be refused at all (root on Linux reads everything).
fn can_lock(dir: &Path) -> bool {
    let probe = dir.join("probe");
    std::fs::create_dir_all(&probe).unwrap();
    let readable = {
        let _g = Locked::new(&probe);
        std::fs::read_dir(&probe).is_ok()
    };
    std::fs::remove_dir(&probe).unwrap();
    !readable
}

#[test]
fn a_locked_home_is_a_blind_spot_with_no_names_and_a_readable_one_is_only_counted() {
    let lab = lab();
    if !can_lock(&lab.d) {
        eprintln!("not reachable: this account reads every folder (root)");
        return;
    }
    write(&lab.bea.join("Documents/bea-private-name.txt"), 30);
    write(&lab.bea.join("Public-ish/bea-readable-name.txt"), 40);
    let _g = Locked::new(&lab.bea.join("Documents"));
    let s = run(sized(facts(&lab, true), 0), &mut Answers::default());
    let json = serde_json::to_string(&s).unwrap();
    assert!(!json.contains("bea-private-name"));
    assert!(!json.contains("bea-readable-name"));
    assert!(!json.contains("Public-ish"));
    assert!(s.blind_spots.iter().any(|b| b.reason
        == BlindReason::AnotherPerson {
            account_id: Some(BEA.into())
        }
        && b.path.is_none()));
    // Her readable part is counted toward the drive, not listed.
    let bea = s
        .places
        .iter()
        .find(|p| {
            p.placement.whose
                == Whose::Person {
                    account_id: BEA.into(),
                }
        })
        .unwrap();
    assert_eq!(bea.bytes, 40);
    assert_ne!(s.top_line, TopLine::Everything);
}

#[test]
fn a_folder_that_cannot_be_read_is_never_shown_as_empty() {
    let lab = lab();
    if !can_lock(&lab.d) {
        eprintln!("not reachable: this account reads every folder (root)");
        return;
    }
    write(&lab.d.join("Work/plan.txt"), 30);
    let _g = Locked::new(&lab.d.join("Work"));
    let s = run(sized(facts(&lab, false), 0), &mut Answers::default());
    let work = find(&s, FolderRole::Other, "Work");
    assert_eq!(
        work.inclusion,
        Inclusion::LeftOut {
            reason: LeftOutReason::Unreadable
        }
    );
    let spot = s
        .blind_spots
        .iter()
        .find(|b| b.path.as_deref().is_some_and(|p| p.ends_with("Work")))
        .expect("listed as a blind spot");
    assert_eq!(spot.reason, BlindReason::NoPermission);
    assert_eq!(s.top_line, TopLine::AccountedForExcept { places: 1 });
}

#[test]
fn links_and_junctions_are_never_followed() {
    let lab = lab();
    write(&lab.d.join("Real/a.txt"), 5);
    let loop_dir = lab.d.join("Real/loop");
    let made = make_dir_link(&lab.d, &loop_dir);
    if !made {
        eprintln!("not reachable: this system would not make a folder link");
        return;
    }
    let s = run(sized(facts(&lab, false), 0), &mut Answers::default());
    assert!(s.finished);
    assert!(s.links >= 1);
    assert!(!s.record.items.iter().any(|i| shown(i).contains("loop/")));
}

fn make_dir_link(target: &Path, link: &Path) -> bool {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link).is_ok()
    }
    #[cfg(windows)]
    {
        std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()
            .is_ok_and(|o| o.status.success())
    }
}

#[test]
fn a_file_with_two_names_is_counted_once() {
    let lab = lab();
    write(&lab.d.join("Music/song.mp3"), 2 * 1024 * 1024);
    if std::fs::hard_link(
        lab.d.join("Music/song.mp3"),
        lab.d.join("Music/song-again.mp3"),
    )
    .is_err()
    {
        eprintln!("not reachable: no hard links here");
        return;
    }
    let s = run(sized(facts(&lab, false), 0), &mut Answers::default());
    let d = &s.volumes[1];
    assert_eq!(d.seen_bytes, 2 * 1024 * 1024);
}

#[test]
fn space_nobody_can_explain_is_a_blind_spot_and_small_gaps_are_not() {
    let lab = lab();
    write(&lab.d.join("Photos/a.jpg"), 10);
    let gb = 1024 * 1024 * 1024u64;
    let s = run(sized(facts(&lab, false), 6 * gb), &mut Answers::default());
    assert!(s.volumes.iter().any(|v| v.flagged));
    assert!(
        s.blind_spots
            .iter()
            .any(|b| matches!(b.reason, BlindReason::Unexplained { bytes } if bytes == 6 * gb))
    );
    assert_eq!(s.top_line, TopLine::AccountedForExcept { places: 2 });
    let s = run(sized(facts(&lab, false), 4 * gb), &mut Answers::default());
    assert!(!s.volumes.iter().any(|v| v.flagged));
    assert_eq!(s.top_line, TopLine::Everything, "{:#?}", s.blind_spots);
}

#[test]
fn removable_network_and_disc_drives_are_listed_not_walked() {
    let lab = lab();
    let usb = lab.c.parent().unwrap().join("USB");
    write(&usb.join("holiday.mov"), 10);
    let mut f = sized(facts(&lab, false), 0);
    f.volumes.push(volume(
        &usb,
        false,
        VolumeKind::Removable,
        DiskKind::Unknown,
    ));
    f.volumes.push(volume(
        Path::new("Z:\\"),
        false,
        VolumeKind::Network,
        DiskKind::Unknown,
    ));
    let s = run(f, &mut Answers::default());
    assert!(!s.record.items.iter().any(|i| shown(i).contains("holiday")));
    assert!(!s.volumes[2].walked);
    assert_eq!(s.volumes[2].kind, VolumeKind::Removable);
    assert!(!s.volumes[3].walked);
    assert_eq!(s.top_line, TopLine::Everything, "{:#?}", s.blind_spots);
}

#[test]
fn a_drive_the_person_used_that_is_not_connected_is_a_blind_spot() {
    let lab = lab();
    let mut f = sized(facts(&lab, false), 0);
    let gone = if cfg!(windows) {
        PathBuf::from(r"Q:\Backups\old.zip")
    } else {
        PathBuf::from("/Volumes/PCTwinGone/old.zip")
    };
    f.remembered.push(gone);
    f.volumes.push(volume(
        &lab.c.parent().unwrap().join("Other"),
        false,
        VolumeKind::OtherSystem,
        DiskKind::Unknown,
    ));
    let s = run(f, &mut Answers::default());
    assert!(
        s.blind_spots
            .iter()
            .any(|b| b.reason == BlindReason::NotConnected)
    );
    assert!(
        s.blind_spots
            .iter()
            .any(|b| b.reason == BlindReason::OtherSystem)
    );
    assert_eq!(s.top_line, TopLine::AccountedForExcept { places: 2 });
}

#[test]
fn a_stopped_sweep_says_so_and_resumes_to_the_same_answer() {
    let lab = lab();
    for i in 0..30 {
        write(&lab.d.join(format!("Data/{i}/{i}.txt")), i + 1);
        write(&lab.ada.join(format!("code/{i}/f.rs")), i + 1);
    }
    let whole = run(sized(facts(&lab, false), 0), &mut Answers::default());

    let stop = AtomicBool::new(false);
    let mut saved = None;
    let limits = SweepLimits {
        checkpoint_entries: 5,
        ..SweepLimits::default()
    };
    let first = sweep(
        sized(facts(&lab, false), 0),
        &mut Answers::default(),
        &stop,
        &mut |view| {
            saved = Some(view.to_json());
            stop.store(true, Ordering::SeqCst);
        },
        &limits,
        None,
    );
    assert!(!first.finished);
    assert_eq!(first.top_line, TopLine::DidNotFinish);
    let frontier = pctwin_scan::Frontier::from_json(&saved.expect("a checkpoint")).unwrap();
    assert!(frontier.pending() > 0);
    let go = AtomicBool::new(false);
    let resumed = sweep(
        sized(facts(&lab, false), 0),
        &mut Answers::default(),
        &go,
        &mut |_| {},
        &limits,
        Some(frontier),
    );
    assert!(resumed.finished);
    let ids = |s: &Sweep| {
        let mut v: Vec<_> = s
            .record
            .items
            .iter()
            .map(|i| (i.id, i.size_bytes, i.inclusion))
            .collect();
        v.sort_by_key(|x| x.0);
        v
    };
    assert_eq!(ids(&resumed), ids(&whole));
    assert_eq!(resumed.volumes, whole.volumes);
    assert_eq!(resumed.top_line, whole.top_line);
}

#[test]
fn a_stop_before_anything_is_never_everything() {
    let lab = lab();
    let stop = AtomicBool::new(true);
    let s = sweep(
        sized(facts(&lab, false), 0),
        &mut Answers::default(),
        &stop,
        &mut |_| {},
        &SweepLimits::default(),
        None,
    );
    assert!(!s.finished);
    assert_eq!(s.top_line, TopLine::DidNotFinish);
    assert!(s.frontier.is_some());
}

#[test]
fn a_very_big_folder_is_counted_as_one_item() {
    let lab = lab();
    for i in 0..12 {
        write(&lab.d.join(format!("Huge/{i}.bin")), 10);
    }
    let stop = AtomicBool::new(false);
    let s = sweep(
        sized(facts(&lab, false), 0),
        &mut Answers::default(),
        &stop,
        &mut |_| {},
        &SweepLimits {
            place_items: 5,
            ..SweepLimits::default()
        },
        None,
    );
    let huge = find(&s, FolderRole::Other, "Huge");
    assert_eq!(huge.size_bytes, 120);
    assert!(!s.record.items.iter().any(|i| shown(i).starts_with("Huge/")));
    assert_eq!(s.volumes[1].unexplained_bytes, 0);
}

#[test]
fn the_record_still_checks_out() {
    let lab = lab();
    write(&lab.d.join("Photos/a.jpg"), 10);
    write(&lab.ada.join("Documents/b.txt"), 10);
    let s = run(sized(facts(&lab, false), 0), &mut Answers::default());
    s.record.validate().unwrap();
    let back = pctwin_record::Record::from_json(&s.record.to_json()).unwrap();
    assert_eq!(back, s.record);
}

#[test]
fn a_refusal_keeps_its_reason() {
    use std::io::Error;
    if cfg!(windows) {
        assert_eq!(
            blind_reason_for(&Error::from_raw_os_error(5)),
            Some(BlindReason::NoPermission)
        );
        assert_eq!(
            blind_reason_for(&Error::from_raw_os_error(2)),
            Some(BlindReason::ChangedDuringScan)
        );
        assert_eq!(
            blind_reason_for(&Error::from_raw_os_error(32)),
            Some(BlindReason::InUse)
        );
        assert_eq!(
            blind_reason_for(&Error::from_raw_os_error(-2_144_272_384)),
            Some(BlindReason::LockedDrive)
        );
    } else {
        assert_eq!(
            blind_reason_for(&Error::from_raw_os_error(13)),
            Some(BlindReason::NoPermission)
        );
        assert_eq!(
            blind_reason_for(&Error::from_raw_os_error(2)),
            Some(BlindReason::ChangedDuringScan)
        );
        let eperm = blind_reason_for(&Error::from_raw_os_error(1));
        if cfg!(target_os = "macos") {
            assert_eq!(eperm, Some(BlindReason::MacPrivacy));
        } else {
            assert_eq!(eperm, Some(BlindReason::NoPermission));
        }
    }
    assert_eq!(
        blind_reason_for(&Error::from_raw_os_error(9999)),
        Some(BlindReason::Refused { code: Some(9999) })
    );
}

#[test]
fn a_second_root_behind_the_system_root_is_walked_without_counting_twice() {
    // As macOS's data volume sits behind `/`: what `/` shows (Users) is seen there; what only the
    // data volume holds (a folder an administrator made at its root) is found through `also`.
    let lab = lab();
    let data = lab.c.join("System").join("Volumes").join("Data");
    write(&data.join("Projects/only-here.txt"), 30);
    write(&data.join("Users/also-at-root.txt"), 7);
    write(&lab.c.join("Users/also-at-root.txt"), 7);
    let mut f = sized(facts(&lab, false), 0);
    f.volumes[0].skip.push(lab.c.join("System").join("Volumes"));
    f.volumes[0].also.push(data.clone());
    // macOS lists what the data volume shows through `/` in /usr/share/firmlinks.
    f.volumes[0].skip.push(data.join("Users"));
    f.volumes[0].free_bytes += 7;
    let s = run(f, &mut Answers::default());
    find(
        &s,
        FolderRole::Other,
        "System/Volumes/Data/Projects/only-here.txt",
    );
    assert!(
        !s.record
            .items
            .iter()
            .any(|i| shown(i).starts_with("System/Volumes/Data/Users"))
    );
    assert_eq!(s.volumes[0].unexplained_bytes, 0);
}

#[test]
fn a_folder_among_the_homes_that_nobody_explains_is_counted_by_its_owner_and_never_named() {
    let lab = lab();
    write(&lab.c.join("Users/ghost/Desktop/ghost-plans.txt"), 12);
    write(&lab.c.join("Users/ada.OLD/Documents/old-cv.docx"), 9);
    write(&lab.c.join("Users/Default/NTUSER.DAT"), 5);
    let mut answers = Answers::default();
    answers.0.insert(
        "ada.OLD".into(),
        AccessFact::Known(access_from_sddl(&format!("O:{ADA}D:P(A;OICI;FA;;;{ADA})")).unwrap()),
    );
    answers.0.insert(
        "ghost".into(),
        AccessFact::Known(
            access_from_sddl(
                "O:S-1-5-21-111-222-333-1077D:P(A;OICI;FA;;;S-1-5-21-111-222-333-1077)",
            )
            .unwrap(),
        ),
    );
    for several in [false, true] {
        let s = run(sized(facts(&lab, several), 0), &mut answers);
        // Somebody's old home: counted, never named, never written off as the system's.
        let json = serde_json::to_string(&s).unwrap();
        assert!(!json.contains("ghost-plans"), "several={several}");
        assert!(!json.contains("ghost"), "several={several}");
        let ghost = s
            .places
            .iter()
            // (Bea's empty folder is another unexplained home in the one-person case.)
            .find(|p| matches!(p.placement.whose, Whose::FromOldAccount { .. }) && p.bytes > 0)
            .expect("counted as from an old account");
        // The owner's ID is kept where it is a person's ID on this system (a SID on Windows).
        if cfg!(windows) {
            assert_eq!(
                ghost.placement.whose,
                Whose::FromOldAccount {
                    account_id: Some("S-1-5-21-111-222-333-1077".into())
                }
            );
        }
        assert_eq!(ghost.bytes, 12);
        assert_eq!(ghost.path, None);
        // The mover's own old profile stays the mover's.
        let cv = find(&s, FolderRole::Other, "Users/ada.OLD/Documents/old-cv.docx");
        assert_eq!(cv.owner, mover());
        let default = s
            .places
            .iter()
            .find(|p| p.path.as_deref().is_some_and(|x| x.ends_with("Default")))
            .unwrap();
        assert_eq!(default.placement.whose, Whose::System);
    }
}

#[test]
fn a_folder_whose_access_this_person_may_not_read_is_a_blind_spot_without_its_name() {
    let lab = lab();
    write(&lab.d.join("Theirs-secret-name/x.txt"), 4);
    let mut answers = Answers::default();
    answers.0.insert(
        "Theirs-secret-name".into(),
        AccessFact::Unknown {
            why: "denied".into(),
        },
    );
    let s = run(sized(facts(&lab, true), 0), &mut answers);
    let json = serde_json::to_string(&s).unwrap();
    assert!(!json.contains("Theirs-secret-name"));
    let spot = s
        .blind_spots
        .iter()
        .find(|b| b.reason == BlindReason::AccessNotReadable)
        .expect("a blind spot");
    assert_eq!(spot.path, None);
    assert!(spot.volume.is_some());
    assert_ne!(s.top_line, TopLine::Everything);
}

// --- Regressions from the fresh-context review (16 October 2026) ---------------------------

#[test]
fn a_folder_renamed_during_the_sweep_is_never_counted_as_looked_at() {
    let lab = lab();
    for i in 0..200 {
        write(&lab.d.join(format!("Photos/Album{i}/beach{i}.jpg")), 10);
    }
    let stop = AtomicBool::new(false);
    let mut renamed = false;
    let d = lab.d.clone();
    let s = sweep(
        sized(facts(&lab, false), 0),
        &mut Answers::default(),
        &stop,
        &mut |view| {
            // Once an album waits to be listed (its parent was listed), it moves away.
            if !renamed {
                renamed = rename_a_queued_folder(&view.to_json(), &d.join("Photos"), "Album", 200);
            }
        },
        &SweepLimits {
            checkpoint_entries: 1,
            checkpoint_every: std::time::Duration::ZERO,
            ..SweepLimits::default()
        },
        None,
    );
    assert!(renamed);
    assert!(s.finished);
    assert!(
        s.blind_spots
            .iter()
            .any(|b| b.reason == BlindReason::ChangedDuringScan),
        "{:#?}",
        s.blind_spots
    );
    assert_ne!(s.top_line, TopLine::Everything);
}

#[cfg(windows)]
#[test]
fn a_folder_whose_name_ends_in_a_dot_is_found() {
    let lab = lab();
    let odd = format!(r"\\?\{}\Notes.", lab.d.display());
    std::fs::create_dir_all(&odd).unwrap();
    std::fs::write(format!(r"{odd}\kept.txt"), b"12345").unwrap();
    let s = run(sized(facts(&lab, false), 0), &mut Answers::default());
    find(&s, FolderRole::Other, "Notes./kept.txt");
    assert_eq!(s.top_line, TopLine::Everything, "{:#?}", s.blind_spots);
    std::fs::remove_file(format!(r"{odd}\kept.txt")).unwrap();
    std::fs::remove_dir(&odd).unwrap();
}

#[test]
fn another_persons_names_never_reach_the_frontier_or_a_stopped_sweep() {
    let lab = lab();
    for i in 0..15 {
        write(&lab.bea.join(format!("BEA-PICS-{i}/BEA-PHOTO-{i}.jpg")), 10);
    }
    write(&lab.d.join("Shared/x.txt"), 1);
    let stop = AtomicBool::new(false);
    let mut saved = Vec::new();
    let s = sweep(
        sized(facts(&lab, true), 0),
        &mut Answers::default(),
        &stop,
        &mut |view| {
            saved.push(view.to_json());
            if saved.len() == 3 {
                stop.store(true, Ordering::SeqCst);
            }
        },
        &SweepLimits {
            checkpoint_entries: 1,
            checkpoint_every: std::time::Duration::ZERO,
            ..SweepLimits::default()
        },
        None,
    );
    for json in &saved {
        assert!(!String::from_utf8_lossy(json).contains("BEA-"));
    }
    assert!(!serde_json::to_string(&s).unwrap().contains("BEA-"));
    // Resumed, the sweep still counts her home, still without a name.
    let resumed = sweep(
        sized(facts(&lab, true), 0),
        &mut Answers::default(),
        &AtomicBool::new(false),
        &mut |_| {},
        &SweepLimits::default(),
        s.frontier.clone(),
    );
    assert!(resumed.finished);
    assert!(!serde_json::to_string(&resumed).unwrap().contains("BEA-"));
    let bea = resumed
        .places
        .iter()
        .find(|p| {
            p.placement.whose
                == Whose::Person {
                    account_id: BEA.into(),
                }
        })
        .unwrap();
    assert_eq!(bea.bytes, 150);
}

#[test]
fn another_persons_project_history_or_package_in_a_shared_folder_is_not_listed() {
    let lab = lab();
    write(&lab.d.join("Family/holiday.jpg"), 3);
    write(&lab.d.join("Family/.git/BEA-COMMIT"), 3);
    write(&lab.d.join("Family/BEA-DIARY.pages/Index.xml"), 3);
    write(&lab.d.join("Family/node_modules/BEA-PKG/index.js"), 3);
    let bea_only = || {
        AccessFact::Known(
            access_from_sddl(&format!("O:{BEA}D:P(A;;FA;;;BA)(A;OICI;FA;;;{BEA})")).unwrap(),
        )
    };
    let mut answers = Answers::default();
    for name in [".git", "BEA-DIARY.pages", "node_modules"] {
        answers.0.insert(name.into(), bea_only());
    }
    let s = run(sized(facts(&lab, true), 0), &mut answers);
    assert_eq!(
        find(&s, FolderRole::Other, "Family/holiday.jpg").owner,
        Owner::Shared
    );
    let json = serde_json::to_string(&s).unwrap();
    assert!(!json.contains("BEA-"), "{json}");
    assert!(!s.record.items.iter().any(|i| shown(i).contains(".git")));
    assert!(
        !s.record
            .items
            .iter()
            .any(|i| shown(i).contains("node_modules"))
    );
    let theirs = s
        .blind_spots
        .iter()
        .find(|b| {
            b.reason
                == BlindReason::AnotherPerson {
                    account_id: Some(BEA.into()),
                }
        })
        .expect("a blind spot for her corners");
    assert_eq!(theirs.path, None);
    assert_eq!(theirs.count, 3);
}

#[test]
fn a_subfolder_everyone_can_read_inside_a_place_shared_by_two_stays_shared_by_two() {
    let lab = lab();
    write(&lab.d.join("Ours/Open/note.txt"), 3);
    let mut answers = Answers::default();
    answers.0.insert(
        "Ours".into(),
        AccessFact::Known(
            access_from_sddl(&format!("O:{ADA}D:P(A;OICI;FA;;;{ADA})(A;OICI;FA;;;{BEA})")).unwrap(),
        ),
    );
    let s = run(sized(facts(&lab, true), 0), &mut answers);
    let note = find(&s, FolderRole::Other, "Ours/Open/note.txt");
    assert_eq!(
        s.placement_of(&note.id).unwrap().whose,
        Whose::Shared {
            limited_to: vec![ADA.into(), BEA.into()]
        }
    );
}

#[test]
fn a_home_that_is_its_own_volume_is_walked_once_and_as_that_persons_home() {
    let lab = lab();
    write(&lab.ada.join("code/main.rs"), 7);
    write(&lab.bea.join("BEA-SECRET/letter.txt"), 9);
    let mut f = sized(facts(&lab, true), 0);
    // As a ZFS dataset or a systemd-homed home per person: each home is its own mount.
    f.volumes[0].skip.push(lab.ada.clone());
    f.volumes[0].skip.push(lab.bea.clone());
    f.volumes
        .push(volume(&lab.ada, false, VolumeKind::Internal, DiskKind::Ssd));
    f.volumes
        .push(volume(&lab.bea, false, VolumeKind::Internal, DiskKind::Ssd));
    let s = run(f, &mut Answers::default());
    let main: Vec<_> = s
        .record
        .items
        .iter()
        .filter(|i| shown(i).ends_with("main.rs"))
        .collect();
    assert_eq!(main.len(), 1, "listed once");
    assert_eq!(main[0].owner, mover());
    assert_eq!(main[0].place.role, FolderRole::Home);
    assert!(!serde_json::to_string(&s).unwrap().contains("BEA-SECRET"));
    let bea = s
        .places
        .iter()
        .find(|p| {
            p.placement.whose
                == Whose::Person {
                    account_id: BEA.into(),
                }
        })
        .unwrap();
    assert_eq!(bea.bytes, 9);
}

#[cfg(windows)]
#[test]
fn the_old_homes_inside_an_older_copy_of_windows_are_placed_by_their_owners() {
    let lab = lab();
    write(&lab.c.join("Windows.old/Windows/System32/old.dll"), 50);
    write(
        &lab.c
            .join("Windows.old/Users/olduser/Documents/OLD-LETTER.docx"),
        20,
    );
    let mut answers = Answers::default();
    answers.0.insert(
        "olduser".into(),
        AccessFact::Known(
            access_from_sddl("O:S-1-5-21-9-9-9-1001D:P(A;OICI;FA;;;S-1-5-21-9-9-9-1001)").unwrap(),
        ),
    );
    let s = run(sized(facts(&lab, false), 0), &mut answers);
    assert!(!serde_json::to_string(&s).unwrap().contains("OLD-LETTER"));
    let old = s
        .places
        .iter()
        .find(|p| matches!(p.placement.whose, Whose::FromOldAccount { .. }) && p.bytes == 20)
        .expect("the old home is counted as from an old account");
    assert_eq!(old.path, None);
}

#[test]
fn local_groups_and_gone_accounts_do_not_widen_a_folder() {
    let lab = lab();
    write(&lab.d.join("Mine/a.txt"), 3);
    let mut answers = Answers::default();
    answers.0.insert(
        "Mine".into(),
        AccessFact::Known(
            access_from_sddl(&format!(
                "O:{ADA}D:P(A;OICI;FA;;;{ADA})(A;OICI;FA;;;S-1-5-21-111-222-333-1077)(A;OICI;FA;;;S-1-5-21-111-222-333-1012)"
            ))
            .unwrap(),
        ),
    );
    let s = run(sized(facts(&lab, true), 0), &mut answers);
    assert_eq!(find(&s, FolderRole::Other, "Mine/a.txt").owner, mover());
}

#[test]
fn two_blind_spots_that_cannot_be_named_still_count_as_two() {
    let lab = lab();
    write(&lab.d.join("Theirs-one/x.txt"), 1);
    write(&lab.d.join("Theirs-two/y.txt"), 1);
    let mut answers = Answers::default();
    for n in ["Theirs-one", "Theirs-two"] {
        answers.0.insert(
            n.into(),
            AccessFact::Unknown {
                why: "denied".into(),
            },
        );
    }
    let s = run(sized(facts(&lab, true), 0), &mut answers);
    assert_eq!(s.top_line, TopLine::AccountedForExcept { places: 2 });
    let spot = s
        .blind_spots
        .iter()
        .find(|b| b.reason == BlindReason::AccessNotReadable)
        .unwrap();
    assert_eq!(spot.count, 2);
}

#[test]
fn a_slow_checkpoint_never_stops_the_sweep_from_finishing() {
    let lab = lab();
    for i in 0..40 {
        write(&lab.d.join(format!("Data/{i}/f.txt")), 1);
    }
    let mut saves = 0;
    let s = sweep(
        sized(facts(&lab, false), 0),
        &mut Answers::default(),
        &AtomicBool::new(false),
        &mut |_| {
            saves += 1;
            std::thread::sleep(std::time::Duration::from_millis(30));
        },
        &SweepLimits {
            checkpoint_entries: 1,
            checkpoint_every: std::time::Duration::ZERO,
            ..SweepLimits::default()
        },
        None,
    );
    assert!(s.finished);
    // Saving takes at most about a fifth of the time: far fewer saves than folders.
    assert!(saves < 30, "{saves} saves");
}

#[cfg(unix)]
#[test]
fn a_file_one_person_keeps_to_themselves_in_a_shared_folder_is_not_listed() {
    let lab = lab();
    write(&lab.d.join("Family/holiday.jpg"), 3);
    write(&lab.d.join("Family/BEA-PRIVATE.txt"), 4);
    let mut answers = Answers::default();
    answers.0.insert(
        "BEA-PRIVATE.txt".into(),
        AccessFact::Known(access_from_sddl(&format!("O:{BEA}D:P(A;;FA;;;{BEA})")).unwrap()),
    );
    let s = run(sized(facts(&lab, true), 0), &mut answers);
    find(&s, FolderRole::Other, "Family/holiday.jpg");
    assert!(!serde_json::to_string(&s).unwrap().contains("BEA-PRIVATE"));
}

// --- Regressions from the second fresh-context review (17 October 2026) --------------------

const CARL: &str = "S-1-5-21-111-222-333-1003";

fn three(lab: &Lab) -> SweepFacts {
    let mut f = facts(lab, true);
    let carl = lab.c.join("Users").join("carl");
    std::fs::create_dir_all(&carl).unwrap();
    f.census
        .residents
        .push(resident(CARL, PersonKind::Other, &carl));
    f
}

#[test]
fn a_folder_two_others_share_inside_a_place_shared_by_two_lands_only_with_the_people_both_allow() {
    let lab = lab();
    write(&lab.d.join("Ours/AdaCarl/plan.txt"), 3);
    write(&lab.d.join("Ours/CarlOnly/CARL-SECRET.txt"), 3);
    let sddl = |ids: &[&str]| {
        let aces: String = ids.iter().map(|id| format!("(A;OICI;FA;;;{id})")).collect();
        AccessFact::Known(access_from_sddl(&format!("O:{ADA}D:P{aces}")).unwrap())
    };
    let mut answers = Answers::default();
    answers.0.insert("Ours".into(), sddl(&[ADA, BEA]));
    answers.0.insert("AdaCarl".into(), sddl(&[ADA, CARL]));
    answers.0.insert("CarlOnly".into(), sddl(&[CARL]));
    let s = run(sized(three(&lab), 0), &mut answers);
    // Bea could not open it before, so she never gets it; Ada could, so it is hers.
    let plan = find(&s, FolderRole::Other, "Ours/AdaCarl/plan.txt");
    assert_eq!(plan.owner, mover());
    assert_eq!(s.placement_of(&plan.id).unwrap().whose, Whose::Mover);
    assert!(!serde_json::to_string(&s).unwrap().contains("CARL-SECRET"));
    assert!(!serde_json::to_string(&s).unwrap().contains("CarlOnly"));
}

#[test]
fn a_system_folder_that_is_its_own_volume_is_still_the_systems() {
    let lab = lab();
    let system_dir = if cfg!(windows) {
        "Windows"
    } else if cfg!(target_os = "macos") {
        "System"
    } else {
        "var"
    };
    let mounted = lab.c.join(system_dir);
    write(&mounted.join("log/syslog"), 40);
    let mut f = sized(facts(&lab, true), 0);
    f.volumes[0].skip.push(mounted.clone());
    let mut inner = volume(&mounted, false, VolumeKind::Internal, DiskKind::Ssd);
    inner.total_bytes = 1 << 40;
    inner.free_bytes = (1 << 40) - 40;
    f.volumes.push(inner);
    let s = run(f, &mut Answers::default());
    assert!(!s.record.items.iter().any(|i| shown(i).contains("syslog")));
    let place = s
        .places
        .iter()
        .find(|p| p.path.as_deref().is_some_and(|x| x.ends_with(system_dir)))
        .expect("counted as the system's");
    assert_eq!(place.placement.whose, Whose::System);
    assert_eq!(place.bytes, 40);
}

#[test]
fn volumes_that_share_one_file_systems_space_are_judged_together() {
    let lab = lab();
    write(&lab.c.join("Projects/a.txt"), 10);
    write(&lab.d.join("Photos/b.jpg"), 20);
    let gb = 1024 * 1024 * 1024u64;
    let mut f = facts(&lab, false);
    // A megabyte of room over the 6 GB: macOS and Linux count whole disk blocks, a little more
    // than the files' own sizes.
    let used = du(&lab.c) + du(&lab.d) + 6 * gb + (1 << 20);
    for v in &mut f.volumes {
        v.total_bytes = 1 << 40;
        v.free_bytes = (1 << 40) - used;
        v.file_system = "btrfs".into();
    }
    // Two NTFS drives that are the same size and equally full are two drives.
    let mut apart = f.clone();
    for v in &mut apart.volumes {
        v.file_system = "NTFS".into();
    }
    let a = run(apart, &mut Answers::default());
    assert_eq!(a.volumes[1].same_space_as, None);
    let s = run(f, &mut Answers::default());
    let gaps: Vec<_> = s
        .blind_spots
        .iter()
        .filter(|b| matches!(b.reason, BlindReason::Unexplained { .. }))
        .collect();
    assert_eq!(gaps.len(), 1, "{gaps:#?}");
    // (macOS and Linux count what files take on the disk, so the gap is used minus that.)
    let seen = s.volumes[0].seen_bytes + s.volumes[1].seen_bytes;
    assert!(
        matches!(gaps[0].reason, BlindReason::Unexplained { bytes } if bytes == used - seen),
        "{gaps:#?}"
    );
    assert!(used - seen >= 6 * gb);
    assert_eq!(s.volumes[1].same_space_as, Some(s.volumes[0].mount.clone()));
    assert_eq!(s.volumes[1].unexplained_bytes, 0);
}

#[test]
fn a_folder_renamed_inside_an_apps_folder_during_the_sweep_is_a_blind_spot() {
    let lab = lab();
    let apps = if cfg!(windows) {
        "Program Files"
    } else if cfg!(target_os = "macos") {
        "Applications"
    } else {
        "opt"
    };
    for i in 0..200 {
        write(&lab.c.join(apps).join(format!("App{i}/app.bin")), 10);
    }
    let stop = AtomicBool::new(false);
    let mut renamed = false;
    let base = lab.c.join(apps);
    let s = sweep(
        sized(facts(&lab, false), 0),
        &mut Answers::default(),
        &stop,
        &mut |view| {
            if !renamed {
                renamed = rename_a_queued_folder(&view.to_json(), &base, "App", 200);
            }
        },
        &SweepLimits {
            checkpoint_entries: 1,
            checkpoint_every: std::time::Duration::ZERO,
            ..SweepLimits::default()
        },
        None,
    );
    assert!(renamed);
    assert!(
        s.blind_spots
            .iter()
            .any(|b| b.reason == BlindReason::ChangedDuringScan),
        "{:#?}",
        s.blind_spots
    );
    assert_ne!(s.top_line, TopLine::Everything);
}

#[cfg(windows)]
#[test]
fn the_shared_folder_of_an_older_copy_of_windows_is_placed_like_any_folder() {
    let lab = lab();
    write(
        &lab.c.join("Windows.old/Users/Public/Documents/flyer.pdf"),
        8,
    );
    let s = run(sized(facts(&lab, false), 0), &mut Answers::default());
    let flyer = find(
        &s,
        FolderRole::Other,
        "Windows.old/Users/Public/Documents/flyer.pdf",
    );
    assert_eq!(flyer.owner, mover());
}

#[cfg(windows)]
#[test]
fn a_folder_whose_name_ends_in_a_dot_inside_a_special_folder_is_found() {
    let lab = lab();
    let docs = lab.ada.join("Documents");
    let odd = format!(r"\\?\{}\Drafts.", docs.display());
    std::fs::create_dir_all(&odd).unwrap();
    std::fs::write(format!(r"{odd}\draft.txt"), b"123").unwrap();
    let s = run(sized(facts(&lab, false), 0), &mut Answers::default());
    find(&s, FolderRole::Documents, "Drafts./draft.txt");
    assert_eq!(s.top_line, TopLine::Everything, "{:#?}", s.blind_spots);
    std::fs::remove_file(format!(r"{odd}\draft.txt")).unwrap();
    std::fs::remove_dir(&odd).unwrap();
}

#[cfg(windows)]
#[test]
fn a_drive_written_with_forward_slashes_is_still_listed() {
    let lab = lab();
    write(&lab.d.join("Photos/a.jpg"), 5);
    let mut f = sized(facts(&lab, false), 0);
    f.volumes[1].mount = PathBuf::from(lab.d.to_string_lossy().replace('\\', "/"));
    let s = run(f, &mut Answers::default());
    assert!(s.record.items.iter().any(|i| shown(i).ends_with("a.jpg")));
    assert_eq!(s.top_line, TopLine::Everything, "{:#?}", s.blind_spots);
}

/// Renames one folder named `<prefix><n>` that the frontier shows still waiting to be listed.
fn rename_a_queued_folder(frontier: &[u8], base: &Path, prefix: &str, count: usize) -> bool {
    let text = String::from_utf8_lossy(frontier);
    for i in (0..count).rev() {
        let name = format!("{prefix}{i}");
        if text.contains(&format!("\"{name}\"")) && base.join(&name).is_dir() {
            return std::fs::rename(base.join(&name), base.join(format!("Moved-{name}"))).is_ok();
        }
    }
    false
}

// --- Regressions from the third fresh-context review (17 October 2026) ---------------------

#[test]
fn a_system_folder_mounted_apart_that_holds_the_homes_still_holds_homes() {
    // Fedora Silverblue and other image-based systems: /var is its own file system and the homes
    // live in /var/home.
    let lab = lab();
    let system_dir = if cfg!(windows) {
        "Windows"
    } else if cfg!(target_os = "macos") {
        "System"
    } else {
        "var"
    };
    let mounted = lab.c.join(system_dir);
    let homes = mounted.join("home");
    let ada = homes.join("ada");
    let bea = homes.join("bea");
    write(&ada.join("code/main.rs"), 7);
    write(&bea.join("BEA-SECRET/letter.txt"), 9);
    write(&mounted.join("log/syslog"), 40);
    let mut f = facts(&lab, true);
    f.census.profiles_root = Some(homes.clone());
    f.census.residents = vec![
        resident(ADA, PersonKind::Me, &ada),
        resident(BEA, PersonKind::Other, &bea),
    ];
    let mut f = sized(f, 0);
    f.volumes[0].skip.push(mounted.clone());
    let mut inner = volume(&mounted, false, VolumeKind::Internal, DiskKind::Ssd);
    inner.total_bytes = 1 << 40;
    inner.free_bytes = (1 << 40) - du(&mounted);
    f.volumes.push(inner);
    let s = run(f, &mut Answers::default());
    let main = find(&s, FolderRole::Home, "code/main.rs");
    assert_eq!(main.owner, mover());
    assert!(!serde_json::to_string(&s).unwrap().contains("BEA-SECRET"));
    assert!(!s.record.items.iter().any(|i| shown(i).contains("syslog")));
    assert!(s.places.iter().any(|p| p.placement.whose
        == Whose::Person {
            account_id: BEA.into()
        }
        && p.bytes == 9));
}

#[test]
fn the_movers_own_folder_inside_a_place_two_others_share_is_the_movers() {
    let lab = lab();
    write(&lab.d.join("Theirs/AdaCorner/mine.txt"), 3);
    let sddl = |ids: &[&str]| {
        let aces: String = ids.iter().map(|id| format!("(A;OICI;FA;;;{id})")).collect();
        AccessFact::Known(access_from_sddl(&format!("O:{ADA}D:P{aces}")).unwrap())
    };
    let mut answers = Answers::default();
    answers.0.insert("Theirs".into(), sddl(&[BEA, CARL]));
    answers.0.insert("AdaCorner".into(), sddl(&[ADA]));
    let s = run(sized(three(&lab), 0), &mut answers);
    let mine = find(&s, FolderRole::Other, "Theirs/AdaCorner/mine.txt");
    assert_eq!(mine.owner, mover());
}

#[test]
fn a_disk_mounted_inside_a_folder_two_people_share_stays_theirs() {
    let lab = lab();
    let disk = lab.d.join("Ours").join("Disk");
    write(&disk.join("photo.jpg"), 5);
    let mut answers = Answers::default();
    answers.0.insert(
        "Ours".into(),
        AccessFact::Known(
            access_from_sddl(&format!("O:{ADA}D:P(A;OICI;FA;;;{ADA})(A;OICI;FA;;;{BEA})")).unwrap(),
        ),
    );
    let mut f = sized(facts(&lab, true), 0);
    f.volumes[1].skip.push(disk.clone());
    let mut inner = volume(&disk, false, VolumeKind::Internal, DiskKind::Ssd);
    inner.total_bytes = 1 << 40;
    inner.free_bytes = (1 << 40) - 5;
    f.volumes.push(inner);
    let s = run(f, &mut answers);
    let photo = s
        .record
        .items
        .iter()
        .find(|i| shown(i) == "photo.jpg")
        .expect("listed once, from the disk itself");
    assert_eq!(
        s.placement_of(&photo.id).unwrap().whose,
        Whose::Shared {
            limited_to: vec![ADA.into(), BEA.into()]
        }
    );
}

#[test]
fn a_path_with_a_dot_part_is_still_listed() {
    let lab = lab();
    write(&lab.d.join("Photos/a.jpg"), 5);
    let mut f = sized(facts(&lab, false), 0);
    f.volumes[1].mount = lab.d.join(".");
    let s = run(f, &mut Answers::default());
    assert!(s.record.items.iter().any(|i| shown(i).ends_with("a.jpg")));
}

#[test]
fn a_sweep_stopped_again_and_again_during_another_persons_count_finishes_with_the_same_answer() {
    let lab = lab();
    for i in 0..120 {
        write(&lab.bea.join(format!("BEA-{i}/BEA-{i}.jpg")), 10);
    }
    write(&lab.d.join("Shared/x.txt"), 1);
    let whole = sweep(
        sized(facts(&lab, true), 0),
        &mut Answers::default(),
        &AtomicBool::new(false),
        &mut |_| {},
        &SweepLimits::default(),
        None,
    );
    let bea_of = |s: &Sweep| {
        s.places
            .iter()
            .find(|p| {
                p.placement.whose
                    == Whose::Person {
                        account_id: BEA.into(),
                    }
            })
            .map(|p| (p.files, p.bytes))
    };
    // Each run is stopped as soon as a save shows Bea's count under way.
    let limits = SweepLimits {
        checkpoint_entries: 1,
        checkpoint_every: std::time::Duration::ZERO,
        count_pause: std::time::Duration::from_millis(5),
        ..SweepLimits::default()
    };
    let mut frontier = None;
    let mut stopped_mid_count = 0;
    let mut runs = 0;
    let done = loop {
        runs += 1;
        assert!(runs < 60, "the sweep never finished");
        let stop = AtomicBool::new(false);
        let s = sweep(
            sized(facts(&lab, true), 0),
            &mut Answers::default(),
            &stop,
            &mut |view| {
                let json = String::from_utf8_lossy(&view.to_json()).into_owned();
                assert!(!json.contains("BEA-"));
                if json.contains("\"whole_at\":{") {
                    stop.store(true, Ordering::SeqCst);
                }
            },
            &limits,
            frontier.take(),
        );
        if s.finished {
            break s;
        }
        // A stop that lands while the count runs keeps how far it got. (The count may also
        // finish just as the stop lands; that run simply has nothing to keep.)
        let saved = serde_json::to_string(s.frontier.as_ref().unwrap()).unwrap();
        if saved.contains("\"whole_at\":{") {
            stopped_mid_count += 1;
        }
        frontier = s.frontier;
    };
    assert!(stopped_mid_count >= 2, "the count was not stopped partway");
    assert_eq!(bea_of(&done), bea_of(&whole));
    assert_eq!(done.volumes, whole.volumes);
    assert_eq!(done.top_line, whole.top_line);
}
