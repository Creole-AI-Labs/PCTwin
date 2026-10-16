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
    write(&lab.c.join("Windows/System32/kernel.dll"), 5000);
    write(&lab.c.join("Program Files/App/app.exe"), 3000);
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
    write(&lab.ada.join("AppData/Local/Google/Chrome/prefs"), 60);
    write(&lab.ada.join("AppData/Local/Temp/junk.tmp"), 15);
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
        .find(|p| p.path.as_deref().is_some_and(|x| x.ends_with("Windows")))
        .unwrap();
    assert_eq!(windows.placement.whose, Whose::System);
    assert!(matches!(&windows.placement.rule, Rule::RulesFile { row } if row.contains("Windows")));
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
    assert_eq!(s.top_line, TopLine::Everything);
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
    assert_eq!(s.top_line, TopLine::Everything);
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
    assert_eq!(s.top_line, TopLine::Everything);
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
        assert_eq!(blind_reason_for(&Error::from_raw_os_error(2)), None);
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
        assert_eq!(blind_reason_for(&Error::from_raw_os_error(2)), None);
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
