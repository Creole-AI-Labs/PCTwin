//! The full move end to end on one machine, both laptops in one test over an in-memory link,
//! through the same engine the app runs (Phase 2 "joining" proof, 27 October 2026):
//!
//! 1. the old laptop finds its files, its apps with who made each, and its settings (only read),
//!    and sends them with the move's extras (move protocol 6);
//! 2. the new laptop plans every app, setting and special category, the person approves the one
//!    plan (its digest names exactly what was shown), and the files land through the safety gate;
//! 3. the settings are applied through the executor and the effect journal, the apps through an
//!    installer (a fake here: nothing is installed on this machine, nothing runs elevated), and
//!    every item ends with one of the six result words;
//! 4. the person changes one moved setting afterwards; undo puts the rest back and keeps theirs.
//!
//! Settings live in a test hive: on Windows a key of PCTwin's own under
//! `HKEY_CURRENT_USER\Software\PCTwinLab` (removed at the end), never a real setting; elsewhere
//! in memory.
// Tests make files to test on.
#![allow(clippy::disallowed_methods)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use pctwin_apps::routes::{Target, WingetBy};
use pctwin_fullmove::extras::{Extras, OldFolder, from_discovery};
use pctwin_fullmove::plan::{Action, FullPlan, Kind, NewLaptop, Promise, SPECIAL_NOT_YET, Title};
use pctwin_fullmove::run::{AppResult, Installer, Progress, rebuild};
use pctwin_fullmove::undo::{self, UndoPlan};
use pctwin_gate::{Approved, Destinations};
use pctwin_journal::{Actor, Journal, Permission, Rebuild, RestorePoint, Scope};
use pctwin_move::landing::LandingRules;
use pctwin_move::{Live, Preview, Steps, Stop};
use pctwin_recipe::{Arch, Os, Platform};
use pctwin_record::{
    AppArch, AppList, AppMaker, AppSystem, FolderRole, FoundApp, FoundWhere, Inclusion, Item,
    ItemId, ItemKind, ItemName, ItemPath, LaptopId, ManagedBy, Owner, PersonTarget, Place,
    Portability, Record, RegistryView, SignerId, Storage,
};
use pctwin_settings::System;
use pctwin_settings::catalogue::Catalogue;
use pctwin_settings::found::discover_registry;
use pctwin_settings::mechanism::Mechanisms;
use pctwin_settings::registry::{Hive, RegValue};
use pctwin_settings::{SettingsAdapter, Store};
use pctwin_transfer::{Channel, ChannelError, LandingPlace, ReceiveOutcome, role_label};
use pctwin_types::{ItemStatus, Reason};
use tokio::sync::mpsc;

const APP: &str = "Software\\PCTwinLabVendor\\Writer";
const DESKTOP: &str = "Control Panel\\Desktop";
const MOVER: &str = "1000";
const HERE: &str = "1001";

struct Mem {
    tx: mpsc::UnboundedSender<Vec<u8>>,
    rx: mpsc::UnboundedReceiver<Vec<u8>>,
}

impl Channel for Mem {
    async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
        tokio::task::yield_now().await;
        self.tx.send(data.to_vec()).map_err(|_| ChannelError)
    }
    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        self.rx.recv().await.ok_or(ChannelError)
    }
}

fn mem_pair() -> (Mem, Mem) {
    let (a_tx, b_rx) = mpsc::unbounded_channel();
    let (b_tx, a_rx) = mpsc::unbounded_channel();
    (Mem { tx: a_tx, rx: a_rx }, Mem { tx: b_tx, rx: b_rx })
}

fn keys() -> &'static pctwin_grant::RoleKeys {
    static KEYS: std::sync::OnceLock<pctwin_grant::RoleKeys> = std::sync::OnceLock::new();
    KEYS.get_or_init(|| pctwin_grant::RoleKeys::mint().expect("the keys are made once"))
}

/// A test hive: a key of PCTwin's own on Windows, removed when dropped; memory elsewhere.
struct LabHive {
    #[cfg(windows)]
    path: String,
}

impl LabHive {
    fn new(name: &str) -> (Self, Box<dyn Hive>) {
        #[cfg(windows)]
        {
            let path = format!(
                "Software\\PCTwinLab\\{}-{}-{name}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            );
            let hive = pctwin_settings::registry::WinHive::user_subkey(&path).unwrap();
            (Self { path }, Box::new(hive))
        }
        #[cfg(not(windows))]
        {
            let _ = name;
            (
                Self {},
                Box::new(pctwin_settings::registry::MemHive::default()),
            )
        }
    }

    /// Another handle on the same hive.
    #[cfg(windows)]
    fn open(&self) -> Box<dyn Hive> {
        Box::new(pctwin_settings::registry::WinHive::user_subkey(&self.path).unwrap())
    }
}

/// A guard on every system (so the tests hold it the same way everywhere); only Windows' has a
/// key to remove.
impl Drop for LabHive {
    fn drop(&mut self) {
        #[cfg(windows)]
        {
            let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
            let _ = hkcu.delete_subkey_all(&self.path);
        }
    }
}

fn laptop() -> LaptopId {
    LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap()
}

fn mover() -> Owner {
    Owner::Person {
        account_id: MOVER.into(),
    }
}

fn file_item(role: FolderRole, path: &[&str], size: u64) -> Item {
    let place = Place {
        role,
        storage: Storage::SystemDrive,
    };
    let path = ItemPath::new(path.iter().map(|p| ItemName::from_text(p)).collect()).unwrap();
    Item {
        id: ItemId::derive(&laptop(), &mover(), &place, &path),
        kind: ItemKind::File,
        owner: mover(),
        place,
        path,
        size_bytes: size,
        modified_ns: None,
        portability: Portability::Portable,
        managed_by: ManagedBy::Personal,
        download_bytes: None,
        inclusion: Inclusion::Included,
    }
}

fn app(name: &str, maker: AppMaker, code: Option<&str>) -> FoundApp {
    FoundApp {
        name: name.into(),
        version: Some("2.0".into()),
        publisher: Some("PCTwin Lab Ltd".into()),
        owner: Owner::Shared,
        system: AppSystem::Windows,
        found: FoundWhere::WindowsEntry {
            view: RegistryView::Machine,
            key: name.into(),
            product_code: code.map(str::to_string),
        },
        arch: AppArch::UNKNOWN,
        maker,
        web: Some("https://example.org/".into()),
        update_feed: None,
        installers: vec![],
        notes: vec![],
    }
}

fn signed() -> AppMaker {
    AppMaker::Windows(Box::new(SignerId {
        organization: Some("PCTwin Lab Ltd".into()),
        certificate: [4; 32],
        ..Default::default()
    }))
}

const FILES: [(FolderRole, &[&str], &[u8]); 2] = [
    (FolderRole::Documents, &["letter.txt"], b"Dear Ada"),
    (FolderRole::Pictures, &["beach.jpg"], b"sand and sea"),
];

fn folder(root: &Path, role: FolderRole) -> PathBuf {
    root.join(match role {
        FolderRole::Pictures => "Pictures",
        _ => "Documents",
    })
}

fn source_in(root: &Path, item: &Item) -> PathBuf {
    let mut p = folder(root, item.place.role);
    for part in item.path.parts() {
        p.push(part.on_disk());
    }
    p
}

fn lab_removable() -> pctwin_fullmove::run::Removable {
    use pctwin_helper::broker::actions::{
        Hex32, InstallDate, InstalledProduct, ProductCode, ProductVersion,
    };
    pctwin_fullmove::run::Removable {
        product: InstalledProduct {
            code: ProductCode::parse("{3B0346D3-155D-43E5-9561-B9BE51CF2EB6}").unwrap(),
            version: ProductVersion::parse("2.1.0").unwrap(),
            installed_on: InstallDate::parse("20261028").unwrap(),
        },
        installed_by: Hex32([7; 32]),
    }
}

/// The administrator helper's removals, as a stand-in: what undo asked it to remove.
#[derive(Default)]
struct LabApps {
    looked: Vec<String>,
    moves: Vec<[u8; 16]>,
    removed: Vec<pctwin_fullmove::run::RemovableApp>,
}

impl undo::AppUndo for LabApps {
    fn plan(&mut self, app: &pctwin_fullmove::run::RemovableApp) -> UndoPlan {
        self.looked.push(app.name.clone());
        UndoPlan::RemoveApp
    }
    fn remove(
        &mut self,
        for_move: [u8; 16],
        apps: &[pctwin_fullmove::run::RemovableApp],
    ) -> Vec<undo::Result> {
        self.moves.push(for_move);
        self.removed.extend(apps.iter().cloned());
        apps.iter()
            .map(|_| undo::Result::Removed {
                restart_needed: false,
            })
            .collect()
    }
}

/// The installer stand-in: reports what a real install that the laptop then showed would report.
/// Nothing is installed and nothing runs elevated.
struct FakeInstaller {
    asked: Vec<String>,
}

impl Installer for FakeInstaller {
    fn restore_point(&mut self) -> RestorePoint {
        RestorePoint::NotMade {
            why: "the test lab makes no restore point".into(),
        }
    }
    fn install(
        &mut self,
        apps: &[&pctwin_fullmove::plan::AppEntry],
        done: &mut dyn FnMut(),
    ) -> Vec<AppResult> {
        apps.iter()
            .map(|e| {
                self.asked.push(e.winget().unwrap_or_default().to_string());
                done();
                AppResult {
                    result: pctwin_types::ItemResult::done(ItemStatus::Reinstalled).unwrap(),
                    todo: vec![],
                    installed: true,
                    // As the real chain keeps a Windows Installer app the helper recorded.
                    removable: Some(Box::new(lab_removable())),
                }
            })
            .collect()
    }
}

/// The new laptop's person, played by the test: everything to this account, and the plan
/// approved once it is shown with its changes.
struct NewPerson {
    rules: LandingRules,
    extras: Option<Vec<u8>>,
    installed: AppList,
    folders: Vec<(FolderRole, String)>,
    plan: Option<FullPlan>,
    shown_digest: Option<[u8; 32]>,
}

fn winget(a: &FoundApp) -> Vec<(String, WingetBy)> {
    if a.name == "Lab Writer" {
        vec![("PCTwinLab.Writer".into(), WingetBy::ProductCode)]
    } else {
        vec![]
    }
}

impl Steps for NewPerson {
    async fn person(&mut self, _: &Record, _: &str) -> Result<PersonTarget, Stop> {
        Ok(PersonTarget::ExistingAccount {
            account_id: HERE.into(),
        })
    }
    async fn approve(&mut self, record: &Record, _: &Preview) -> Result<(), Stop> {
        let bytes = self.extras.clone().ok_or(Stop::NotApproved)?;
        let x = Extras::decode(&bytes).map_err(|_| Stop::NotApproved)?;
        let digest = pctwin_fullmove::extras::digest(&bytes);
        let here = NewLaptop {
            system: System::Windows,
            target: Target {
                system: AppSystem::Windows,
                arch: AppArch {
                    known: true,
                    x86: false,
                    x64: true,
                    arm64: false,
                },
                linux: None,
            },
            installed: &self.installed,
            folders: self.folders.clone(),
            winget: &winget,
        };
        let plan = FullPlan::build(record, MOVER, Some((&x, digest)), &here, &mut |_, _| {
            Some(None)
        });
        self.shown_digest = Some(plan.digest());
        self.plan = Some(plan);
        Ok(())
    }
    fn landing(&self, item: &Item) -> Option<LandingPlace> {
        self.rules.landing(item)
    }
    fn extras(&mut self, bytes: Vec<u8>) {
        self.extras = Some(bytes);
    }
}

fn get(a: &mut SettingsAdapter, key: &str, name: &str) -> Option<RegValue> {
    a.store
        .mechanisms()
        .user
        .as_deref()
        .unwrap()
        .get(key, name)
        .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_full_move_rebuilds_apps_and_settings_honestly_and_undo_keeps_a_later_edit() {
    pctwin_parse::testing::install_readers();
    let old = tempfile::tempdir().unwrap();
    let new = tempfile::tempdir().unwrap();
    let jdir = tempfile::tempdir().unwrap();

    // The old laptop: two files, three apps, and an app's settings in its hive.
    let mut items = Vec::new();
    for (role, path, bytes) in FILES {
        let i = file_item(role, path, bytes.len() as u64);
        let src = source_in(old.path(), &i);
        std::fs::create_dir_all(src.parent().unwrap()).unwrap();
        std::fs::write(&src, bytes).unwrap();
        items.push(i);
    }
    let mut record = Record::new(laptop());
    record.items = items;
    record.apps.found = vec![
        app(
            "Lab Writer",
            signed(),
            Some("{11111111-2222-3333-4444-555555555555}"),
        ),
        app("Lab Unsigned Tool", AppMaker::Unsigned, None),
        app(
            "Lab Already Here",
            signed(),
            Some("{66666666-2222-3333-4444-555555555555}"),
        ),
    ];
    record.validate().unwrap();

    let old_docs = folder(old.path(), FolderRole::Documents);
    let new_docs = folder(new.path(), FolderRole::Documents);
    let (old_hive, mut old_reg) = LabHive::new("old");
    let set = |h: &mut dyn Hive, k: &str, n: &str, v: RegValue| h.set(k, n, &v).unwrap();
    set(&mut *old_reg, APP, "Theme", RegValue::Sz("ocean".into()));
    set(&mut *old_reg, APP, "FontSize", RegValue::Dword(14));
    set(
        &mut *old_reg,
        APP,
        "LastFile",
        RegValue::Sz(old_docs.join("letter.txt").display().to_string()),
    );
    set(
        &mut *old_reg,
        APP,
        "ApiToken",
        RegValue::Sz("eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJsYWIifQ.c2lnbmF0dXJlLWxhYi10ZXN0".into()),
    );
    set(
        &mut *old_reg,
        DESKTOP,
        "WheelScrollLines",
        RegValue::Sz("3".into()),
    );
    // An app's data in the old home (2.5 finds it by general rules, by its content).
    let mail = old.path().join("AppData").join("Roaming").join("LabMail");
    std::fs::create_dir_all(&mail).unwrap();
    std::fs::write(mail.join("folders.json"), br#"{"inbox": 12, "sent": 3}"#).unwrap();
    std::fs::write(mail.join("signature.txt"), b"Ada, from the lab").unwrap();
    // Only read on the old laptop.
    let mut extras = from_discovery(
        System::Windows,
        discover_registry(&*old_reg),
        vec![OldFolder {
            role: FolderRole::Documents,
            path: old_docs.display().to_string(),
        }],
    );
    // The 2.5 scan runs beside the files' scan, with its own budget.
    let scan = pctwin_fullmove::extras::SpecialScan::start_for(
        pctwin_special::roots::System::Windows,
        &mover(),
        old.path(),
    )
    .unwrap();
    let found = scan.wait(&|| false);
    assert!(
        found
            .as_ref()
            .is_some_and(|f| f.lines.iter().any(|s| s.name.contains("LabMail"))),
        "2.5 found the app's data: {found:#?}"
    );
    extras.with_special(found.as_ref());
    let extras_bytes = extras.encode();

    // The new laptop: its folders, one of the apps already, and a value of its own.
    let mut table = Destinations::new();
    for role in [FolderRole::Documents, FolderRole::Pictures] {
        let dir = folder(new.path(), role);
        std::fs::create_dir_all(&dir).unwrap();
        table
            .approve(role_label(role), Approved::MyFolders, &dir)
            .unwrap();
    }
    let (new_hive, mut new_reg) = LabHive::new("new");
    set(
        &mut *new_reg,
        DESKTOP,
        "WheelScrollLines",
        RegValue::Sz("5".into()),
    );
    let journal = Journal::open_with_record_key(
        &jdir.path().join("journal.redb"),
        pctwin_journal::keystore::TagKey::from_bytes([0x5a; 32]),
    )
    .unwrap();
    let rules = LandingRules {
        mover: MOVER.into(),
        roles: [FolderRole::Documents, FolderRole::Pictures]
            .into_iter()
            .map(|r| (r, Storage::SystemDrive))
            .collect(),
        fallback: Default::default(),
        held: Default::default(),
        left_out: Default::default(),
        chosen: Default::default(),
    };
    let mut person = NewPerson {
        rules: rules.clone(),
        extras: None,
        installed: AppList {
            found: vec![app(
                "Lab Already Here",
                signed(),
                Some("{66666666-2222-3333-4444-555555555555}"),
            )],
            not_read: vec![],
        },
        folders: vec![(FolderRole::Documents, new_docs.display().to_string())],
        plan: None,
        shown_digest: None,
    };

    // The move itself, both sides through pctwin-move.
    let (mut a, mut b) = mem_pair();
    let (old_live, new_live) = (Live::default(), Live::default());
    let old_root = old.path().to_path_buf();
    let (sent, received) = tokio::join!(
        async {
            use pctwin_move::greeting::{About, Power, Side, greet};
            let mut link = greet(&mut a, &About::here(Power::Unknown, None), Side::Old)
                .await
                .map_err(pctwin_transfer::TransferError::from)?;
            let who = pctwin_move::Who {
                mover: MOVER,
                people: &[],
                extras: &extras_bytes,
            };
            let mut back = pctwin_move::resume::NoComeback;
            pctwin_move::send_by_rules(
                &mut link,
                &record,
                &who,
                |i| Some(source_in(&old_root, i)),
                &old_live,
                Arc::new(pctwin_transfer::FsOpener),
                &mut back,
                None,
            )
            .await
        },
        async {
            use pctwin_move::greeting::{About, Power, Side, greet};
            let mut link = greet(&mut b, &About::here(Power::Unknown, None), Side::New)
                .await
                .map_err(pctwin_transfer::TransferError::from)?;
            let here = pctwin_move::Here {
                journal: &journal,
                account: HERE,
            };
            let mut redial = pctwin_move::resume::NoRedial;
            pctwin_move::receive_by_rules(
                &mut link,
                &mut person,
                |s, _| s.rules.clone(),
                &mut table,
                here,
                &new_live,
                &mut redial,
            )
            .await
        },
    );
    sent.unwrap();
    let received = received.unwrap();
    // The files: each landed, byte for byte; the old laptop's are unchanged.
    for (f, (role, path, bytes)) in received.files.iter().zip(FILES) {
        assert!(
            matches!(f.outcome, Some(ReceiveOutcome::Finished(_))),
            "{f:?}"
        );
        let landed = folder(new.path(), role).join(path.join("/"));
        assert_eq!(std::fs::read(landed).unwrap(), bytes);
        let original = folder(old.path(), role).join(path.join("/"));
        assert_eq!(std::fs::read(original).unwrap(), bytes);
    }

    // The one plan the person approved, with every change and its undo promise.
    let plan = person
        .plan
        .take()
        .expect("the plan was shown at the approval");
    assert_eq!(person.shown_digest, Some(plan.digest()));
    let changes = plan.changes();
    let line = |name: &str| {
        changes
            .iter()
            .find(|c| c.title == Title::Name(name.into()))
            .unwrap_or_else(|| panic!("{name} is on the plan: {changes:#?}"))
    };
    assert_eq!(line("Lab Writer").action, Action::Install);
    assert_eq!(line("Lab Writer").undo, Promise::RemoveYourself);
    assert_eq!(line("Lab Unsigned Tool").needs, Some(Reason::ManualInstall));
    assert_eq!(line("Lab Already Here").action, Action::LeaveAlone);
    assert!(
        changes
            .iter()
            .any(|c| c.kind == Kind::Settings && c.undo == Promise::CanReverse),
        "{changes:#?}"
    );
    // 2.5 looked: each component is on the plan with its step; one that copies says so (its
    // landing is proven in tests/appdata.rs; here nothing is chosen, so it ends needs-action).
    assert!(
        !changes
            .iter()
            .any(|c| c.title == Title::Key(SPECIAL_NOT_YET))
    );
    assert!(
        changes
            .iter()
            .filter(|c| c.kind == Kind::Special)
            .all(|c| c.special.is_some()),
        "{changes:#?}"
    );
    // The token never left the plan's discovery as something to write.
    assert!(
        plan.settings
            .moves
            .iter()
            .all(|m| m.address.name() != "ApiToken"),
        "a secret is never moved"
    );

    // After the files: apps and settings, journaled.
    let cat = Catalogue::starter().unwrap();
    let mut store = Store::new(
        cat,
        Mechanisms {
            system: System::Windows,
            user: Some(new_reg),
            machine: None,
            runner: None,
            localtime: None,
        },
    );
    store.remember(plan.settings.book()).unwrap();
    let mut adapter = SettingsAdapter {
        store,
        platform: Platform {
            os: Os::Windows,
            arch: Arch::X86_64,
        },
    };
    let mut installer = FakeInstaller { asked: vec![] };
    let rebuild_rec = Rebuild::new(&journal, &keys().install, &keys().setting);
    let scope = Scope::for_a_move().unwrap();
    let actor = Actor {
        acting_account: HERE.into(),
        for_account: HERE.into(),
        permission: Permission::OwnFolders,
    };
    let mut steps = Vec::new();
    // What landed where, from the move's own results: a setting points only at a file there.
    let files = pctwin_fullmove::run::files_landed(
        &plan,
        received.files.iter().filter_map(|f| {
            let item = received.record.items.iter().find(|i| i.id == f.item)?;
            let stored = match &f.outcome {
                Some(ReceiveOutcome::Finished(d)) => d.final_path.clone(),
                Some(ReceiveOutcome::AlreadyThere(s)) => s.clone(),
                _ => return None,
            };
            Some((item, folder(new.path(), item.place.role).join(stored)))
        }),
    );
    let mut done = rebuild(
        &plan,
        &pctwin_fullmove::run::DataDone {
            files,
            ..Default::default()
        },
        &mut installer,
        &mut adapter,
        &rebuild_rec,
        scope,
        &actor,
        LaptopId::from_hex("ffeeddccbbaa99887766554433221100").unwrap(),
        &mut |p| steps.push(p),
    );
    assert_eq!(installer.asked, ["PCTwinLab.Writer"]);
    assert!(steps.contains(&Progress::Permission));
    assert_eq!(done.stopped, None);
    let app_result = |name: &str| {
        done.apps
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, r)| r.result)
    };
    assert_eq!(
        app_result("Lab Writer").map(|r| r.status()),
        Some(ItemStatus::Reinstalled)
    );
    assert_eq!(
        app_result("Lab Unsigned Tool").and_then(|r| r.reason()),
        Some(Reason::ManualInstall)
    );
    assert_eq!(
        app_result("Lab Already Here"),
        None,
        "already here: left alone"
    );
    // Gap 7: an app already here is still said in the results ("already on this laptop, left
    // as it is"), outside the six words: never counted as reinstalled.
    assert_eq!(done.here, vec![("Lab Already Here".to_string(), None)]);
    assert!(!done.undo.apps.contains(&"Lab Already Here".to_string()));
    for (label, _, r) in &done.settings {
        assert_eq!(r.status(), ItemStatus::Converted, "{label}: {r:?}");
    }
    assert!(!done.special.is_empty());
    assert!(
        done.special
            .iter()
            .all(|s| s.result.status() == ItemStatus::NeedsAction),
        "special categories are never claimed copied until their files move: {:?}",
        done.special
    );
    // Every item ends with one of the six words, none of them unverified success.
    assert_eq!(
        done.restore_point,
        Some(RestorePoint::NotMade {
            why: "the test lab makes no restore point".into()
        })
    );
    // What is on the new laptop now.
    assert_eq!(
        get(&mut adapter, APP, "Theme"),
        Some(RegValue::Sz("ocean".into()))
    );
    assert_eq!(
        get(&mut adapter, APP, "FontSize"),
        Some(RegValue::Dword(14))
    );
    assert_eq!(
        get(&mut adapter, APP, "LastFile"),
        Some(RegValue::Sz(
            new_docs.join("letter.txt").display().to_string()
        )),
        "a path into a moved folder points at the moved file"
    );
    assert_eq!(get(&mut adapter, APP, "ApiToken"), None);
    assert_eq!(
        get(&mut adapter, DESKTOP, "WheelScrollLines"),
        Some(RegValue::Sz("3".into()))
    );

    // The person changes one moved setting afterwards.
    adapter
        .store
        .mechanisms()
        .user
        .as_deref_mut()
        .unwrap()
        .set(APP, "Theme", &RegValue::Sz("forest".into()))
        .unwrap();

    // The app PCTwin installed from a Windows Installer package is kept for undo to remove, not
    // listed for the person.
    assert!(done.undo.apps.is_empty(), "{:?}", done.undo.apps);
    assert_eq!(done.undo.removable.len(), 1);
    assert_eq!(done.undo.removable[0].name, "Lab Writer");
    assert_eq!(done.undo.removable[0].removable, lab_removable());
    let mut lab_apps = LabApps::default();
    // Undo, previewed then done.
    let preview = undo::preview(&journal, &done.undo, &mut adapter, &mut lab_apps).unwrap();
    let plan_of = |name: &str| {
        preview
            .iter()
            .find(|l| l.name.contains(name))
            .map(|l| l.plan)
    };
    assert_eq!(
        plan_of("Writer"),
        Some(UndoPlan::KeepChanged),
        "{preview:#?}"
    );
    assert_eq!(
        plan_of("Lab Writer"),
        Some(UndoPlan::RemoveApp),
        "{preview:#?}"
    );
    let lines = undo::undo(
        &rebuild_rec,
        &mut done.undo,
        &mut adapter,
        None,
        &mut lab_apps,
    )
    .unwrap();
    assert!(
        lines
            .iter()
            .any(|l| matches!(l.result, undo::Result::KeptChanged { kept: 1 })),
        "{lines:#?}"
    );
    // The app went to the helper's removal, by the very record the install kept, and is said.
    assert_eq!(lab_apps.looked, ["Lab Writer"]);
    assert_eq!(lab_apps.removed.len(), 1);
    assert_eq!(lab_apps.removed[0].removable, lab_removable());
    assert_eq!(lab_apps.moves, [done.undo.scope]);
    // Review finding F1: the removal that ended is settled in the book; undo again never asks
    // for it, and says it is as before.
    assert_eq!(
        done.undo.removable[0].settled,
        Some(pctwin_fullmove::run::Settled::Removed)
    );
    let preview = undo::preview(&journal, &done.undo, &mut adapter, &mut lab_apps).unwrap();
    assert_eq!(
        preview
            .iter()
            .find(|l| l.name == "Lab Writer")
            .map(|l| l.plan),
        Some(UndoPlan::AlreadyGone)
    );
    let again = undo::undo(
        &rebuild_rec,
        &mut done.undo,
        &mut adapter,
        None,
        &mut lab_apps,
    )
    .unwrap();
    assert_eq!(lab_apps.removed.len(), 1);
    assert_eq!(lab_apps.looked, ["Lab Writer"]);
    assert!(
        again
            .iter()
            .any(|l| l.name == "Lab Writer" && l.result == undo::Result::AsBefore)
    );
    assert!(lines.iter().any(|l| l.name == "Lab Writer"
        && l.result
            == undo::Result::Removed {
                restart_needed: false
            }));
    assert_eq!(
        get(&mut adapter, APP, "Theme"),
        Some(RegValue::Sz("forest".into())),
        "the person's later edit is kept"
    );
    assert_eq!(
        get(&mut adapter, APP, "FontSize"),
        None,
        "put back: not set before"
    );
    assert_eq!(get(&mut adapter, APP, "LastFile"), None);
    assert_eq!(
        get(&mut adapter, DESKTOP, "WheelScrollLines"),
        Some(RegValue::Sz("5".into())),
        "put back to the new laptop's own value"
    );
    // The old laptop's settings are as they were.
    #[cfg(windows)]
    {
        let reread = old_hive.open();
        assert_eq!(
            reread.get(APP, "Theme").unwrap(),
            Some(RegValue::Sz("ocean".into()))
        );
    }
    drop((old_hive, new_hive));
}
