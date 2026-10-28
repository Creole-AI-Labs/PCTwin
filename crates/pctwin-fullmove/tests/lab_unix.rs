//! The full move end to end between two Linux laptops and between two Macs (T26-025, T26-026;
//! Phase 2 "Linux both ways" and the Mac, 28 October 2026), both laptops in one test over an
//! in-memory link, through the same engine the app runs. It runs on every system, so each CI
//! runner (Ubuntu and Fedora's kind of Linux on x64 and Arm, Macs on Apple silicon and Intel)
//! lands the files on its own file system:
//!
//! 1. **Files** (T26-025): names that differ only in letter case or in how an accent is written
//!    land side by side, each byte for byte, under names the new laptop's drive keeps apart; a
//!    program file never lands able to run.
//! 2. **Apps**: on Linux through the system's own package tools (stand-ins here that behave like
//!    `pkcon`, `flatpak` and `snap`; nothing is installed), one prompt for every package, and
//!    `reinstalled` only when the laptop's own list shows the same package from the same maker;
//!    on a Mac every app is the person's to install, said with its reason.
//! 3. **Settings**: GNOME's `dconf` or the Mac's `defaults` (stand-ins), journaled, read back.
//! 4. **App data**: found where the person's own settings put it; what the system runs at sign-in
//!    is never planned to land where it would run (T26-026).
//! 5. **Undo** keeps a setting the person changed after the move and puts the rest back.
// Tests make files to test on.
#![allow(clippy::disallowed_methods)]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use pctwin_apps::routes::{LinuxPackages, Target, WingetBy};
use pctwin_fullmove::extras::{Extras, OldFolder, from_discovery};
use pctwin_fullmove::plan::{FullPlan, NewLaptop};
use pctwin_fullmove::run::{Installer, NoInstaller, rebuild};
use pctwin_fullmove::undo;
use pctwin_fullmove::unix::{LinuxInstaller, Tools as PackageTools};
use pctwin_gate::{Approved, Destinations};
use pctwin_journal::{Actor, Journal, Permission, Rebuild, RestorePoint, Scope};
use pctwin_maker::run::{Call, Ran, RunError, Runner as ToolRunner};
use pctwin_move::landing::LandingRules;
use pctwin_move::{Live, Preview, Steps, Stop};
use pctwin_recipe::{Arch, Os, Platform};
use pctwin_record::{
    AppArch, AppList, AppMaker, AppSystem, FolderRole, FoundApp, FoundWhere, Inclusion, Item,
    ItemId, ItemKind, ItemName, ItemPath, LaptopId, ManagedBy, Owner, PersonTarget, Place,
    Portability, Record, Storage,
};
use pctwin_settings::System;
use pctwin_settings::catalogue::Catalogue;
use pctwin_settings::found::{discover_dconf, discover_mac};
use pctwin_settings::gvariant;
use pctwin_settings::mechanism::Mechanisms;
use pctwin_settings::plist::{self, Plist};
use pctwin_settings::runner::{Output, Runner, Tool};
use pctwin_settings::{SettingsAdapter, Store};
use pctwin_transfer::{Channel, ChannelError, LandingPlace, ReceiveOutcome, role_label};
use pctwin_types::{ItemStatus, Reason};
use tokio::sync::mpsc;

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

/* ---------- `defaults` and `dconf` in memory ---------- */

#[derive(Default)]
struct Prefs {
    /// domain -> key -> value
    prefs: BTreeMap<String, Vec<(String, Plist)>>,
    /// full dconf key -> GVariant text
    dconf: BTreeMap<String, String>,
}

#[derive(Default, Clone)]
struct SettingsTools(Rc<RefCell<Prefs>>);

fn ok(s: &str) -> Result<Output, String> {
    Ok(Output {
        ok: true,
        stdout: s.to_string(),
    })
}

fn fail() -> Result<Output, String> {
    Ok(Output {
        ok: false,
        stdout: String::new(),
    })
}

impl Runner for SettingsTools {
    fn run(&mut self, tool: Tool, args: &[&str]) -> Result<Output, String> {
        let mut s = self.0.borrow_mut();
        match (tool, args) {
            (Tool::Defaults, ["domains"]) => ok(&s
                .prefs
                .keys()
                .filter(|d| *d != "NSGlobalDomain")
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")),
            (Tool::Defaults, ["export", d, "-"]) => {
                let items = s.prefs.get(*d).cloned().unwrap_or_default();
                ok(&format!(
                    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">\n{}\n</plist>\n",
                    plist::to_xml(&Plist::Dict(items)).unwrap()
                ))
            }
            (Tool::Defaults, ["write", d, k, rest @ ..]) => {
                let v = match rest {
                    ["-bool", v] => Plist::Bool(*v == "true"),
                    ["-int", v] => Plist::Integer(v.parse().map_err(|_| "bad")?),
                    ["-float", v] => Plist::Real(v.parse().map_err(|_| "bad")?),
                    ["-string", v] => Plist::String(v.to_string()),
                    [xml] => plist::from_xml(xml)?,
                    _ => return fail(),
                };
                let dom = s.prefs.entry(d.to_string()).or_default();
                dom.retain(|(key, _)| key != k);
                dom.push((k.to_string(), v));
                ok("")
            }
            (Tool::Defaults, ["delete", d, k]) => {
                let dom = s.prefs.entry(d.to_string()).or_default();
                let before = dom.len();
                dom.retain(|(key, _)| key != k);
                if dom.len() == before { fail() } else { ok("") }
            }
            (Tool::Dconf, ["dump", "/"]) => {
                let mut by_dir: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
                for (full, v) in &s.dconf {
                    let (dir, key) = full.rsplit_once('/').unwrap();
                    by_dir
                        .entry(dir.trim_start_matches('/').to_string())
                        .or_default()
                        .push((key.into(), v.clone()));
                }
                let mut out = String::new();
                for (dir, kv) in by_dir {
                    out.push_str(&format!("[{dir}]\n"));
                    for (k, v) in kv {
                        out.push_str(&format!("{k}={v}\n"));
                    }
                    out.push('\n');
                }
                ok(&out)
            }
            (Tool::Dconf, ["read", k]) => ok(&s.dconf.get(*k).cloned().unwrap_or_default()),
            (Tool::Dconf, ["write", k, v]) => {
                gvariant::parse(v)?;
                s.dconf.insert(k.to_string(), v.to_string());
                ok("")
            }
            (Tool::Dconf, ["reset", k]) => {
                s.dconf.remove(*k);
                ok("")
            }
            (Tool::Gsettings, ["writable", ..]) => ok("true"),
            _ => fail(),
        }
    }
}

fn mech(system: System, tools: SettingsTools) -> Mechanisms {
    Mechanisms {
        system,
        user: None,
        machine: None,
        runner: Some(Box::new(tools)),
        localtime: None,
    }
}

/* ---------- `pkcon`, `flatpak` and `snap` ---------- */

/// The system's package tools, as they behave (nothing is installed): each call is recorded, and
/// what an install puts on the laptop's own list is what `after` says for that package.
#[derive(Default, Clone)]
struct PackageStandIn {
    calls: Rc<RefCell<Vec<Vec<String>>>>,
    list: Rc<RefCell<Vec<FoundApp>>>,
    after: Rc<BTreeMap<String, FoundApp>>,
}

impl ToolRunner for PackageStandIn {
    fn run(&self, call: &Call) -> Result<Ran, RunError> {
        let args: Vec<String> = call
            .args
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        let mut line = vec![call.program.to_string_lossy().into_owned()];
        line.extend(args.iter().cloned());
        self.calls.borrow_mut().push(line);
        // Nothing of PCTwin's own environment reaches a tool.
        assert!(call.env.iter().all(|(k, _)| k != "LD_PRELOAD"));
        let done = |out: &str| {
            Ok(Ran {
                code: Some(0),
                stdout: out.into(),
                stderr: String::new(),
            })
        };
        let install = |names: &[String]| {
            for n in names {
                if let Some(a) = self.after.get(n) {
                    self.list.borrow_mut().push(a.clone());
                }
            }
        };
        let a: Vec<&str> = args.iter().map(String::as_str).collect();
        match a.as_slice() {
            ["remotes", "--columns=name,url,options"] => done(
                "flathub\thttps://dl.flathub.org/repo/\tuser\nfakehub\thttps://dl.flathub.org/repo/\tsystem\n",
            ),
            ["--noninteractive", "install", names @ ..] => {
                install(&names.iter().map(|n| n.to_string()).collect::<Vec<_>>());
                done("")
            }
            ["install", "--noninteractive", "-y", "--user", "flathub", id] => {
                install(&[id.to_string()]);
                done("")
            }
            ["install", name] => {
                install(&[name.to_string()]);
                done("")
            }
            _ => Ok(Ran {
                code: Some(1),
                stdout: String::new(),
                stderr: "not a call this stand-in knows".into(),
            }),
        }
    }
}

/* ---------- The two laptops ---------- */

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Linux,
    Mac,
}

impl Kind {
    fn settings(self) -> System {
        match self {
            Kind::Linux => System::Gnome,
            Kind::Mac => System::Mac,
        }
    }
    fn special(self) -> pctwin_special::roots::System {
        match self {
            Kind::Linux => pctwin_special::roots::System::Linux,
            Kind::Mac => pctwin_special::roots::System::Mac,
        }
    }
    fn apps(self) -> AppSystem {
        match self {
            Kind::Linux => AppSystem::Linux,
            Kind::Mac => AppSystem::Mac,
        }
    }
    fn os(self) -> Os {
        match self {
            Kind::Linux => Os::Linux,
            Kind::Mac => Os::Macos,
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

fn file_item(role: FolderRole, name: &str, size: u64) -> Item {
    let place = Place {
        role,
        storage: Storage::SystemDrive,
    };
    let path = ItemPath::new(vec![ItemName::from_text(name)]).unwrap();
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

/// The files: two that differ only in letter case, two that differ only in how the accent is
/// written (one letter "é", or "e" with an accent after it), and a program file.
const FILES: [(&str, &[u8]); 6] = [
    ("letter.txt", b"Dear Ada"),
    ("Report.txt", b"upper case report"),
    ("report.txt", b"lower case report"),
    ("caf\u{e9}.txt", b"one letter"),
    ("cafe\u{301}.txt", b"letter and accent"),
    ("run.sh", b"#!/bin/sh\necho hi\n"),
];

fn docs(root: &Path) -> PathBuf {
    root.join("Documents")
}

fn app(kind: Kind, name: &str, found: FoundWhere, maker: AppMaker) -> FoundApp {
    FoundApp {
        name: name.into(),
        version: Some("2.0".into()),
        publisher: None,
        owner: Owner::Shared,
        system: kind.apps(),
        found,
        arch: AppArch::UNKNOWN,
        maker,
        web: None,
        update_feed: None,
        installers: vec![],
        notes: vec![],
    }
}

fn repo(origin: &str) -> AppMaker {
    AppMaker::Repository {
        origin: origin.into(),
    }
}

fn flathub(key: u8) -> AppMaker {
    AppMaker::Flatpak {
        remote_url: "https://dl.flathub.org/repo/".into(),
        app_id: "org.lab.Paint".into(),
        key: [key; 32],
    }
}

fn paint(kind: Kind, key: u8) -> FoundApp {
    let mut a = app(
        kind,
        "Lab Paint",
        FoundWhere::Flatpak {
            app_id: "org.lab.Paint".into(),
            remote: "flathub".into(),
            remote_url: Some("https://dl.flathub.org/repo/".into()),
            system_wide: false,
        },
        flathub(key),
    );
    a.owner = mover();
    a
}

fn old_apps(kind: Kind) -> Vec<FoundApp> {
    match kind {
        Kind::Linux => vec![
            app(
                kind,
                "lab-writer",
                FoundWhere::Deb {
                    package: "lab-writer".into(),
                },
                repo("http://archive.ubuntu.com/ubuntu noble/main"),
            ),
            paint(kind, 7),
            app(
                kind,
                "lab-notes",
                FoundWhere::Snap {
                    name: "lab-notes".into(),
                },
                repo("snap store: labco"),
            ),
            // Installed from a file: no repository vouches for it.
            app(
                kind,
                "lab-old-tool",
                FoundWhere::Deb {
                    package: "lab-old-tool".into(),
                },
                AppMaker::Unsigned,
            ),
            // A Fedora package: this laptop uses apt, so it has no route here.
            app(
                kind,
                "lab-fedora",
                FoundWhere::Rpm {
                    package: "lab-fedora".into(),
                },
                AppMaker::Rpm {
                    key: "809a8d7ceb10b464".into(),
                },
            ),
        ],
        Kind::Mac => vec![app(
            kind,
            "Lab Writer",
            FoundWhere::MacBundle {
                bundle_id: "org.lab.writer".into(),
                folder: "Lab Writer.app".into(),
                app_store_id: Some(123_456_789),
            },
            AppMaker::MacTeam {
                team_id: "ABCDE12345".into(),
            },
        )],
    }
}

/// What the laptop's own list shows after each install: the writer from a country's mirror of
/// the same archive (the same source), newer; the paint app from the same remote and key; the
/// notes app from another publisher.
fn after_install(kind: Kind) -> BTreeMap<String, FoundApp> {
    let mut writer = app(
        kind,
        "lab-writer",
        FoundWhere::Deb {
            package: "lab-writer".into(),
        },
        repo("http://gb.archive.ubuntu.com/ubuntu noble/main"),
    );
    writer.version = Some("2.1".into());
    let notes = app(
        kind,
        "lab-notes",
        FoundWhere::Snap {
            name: "lab-notes".into(),
        },
        repo("snap store: someone-else"),
    );
    [
        ("lab-writer".to_string(), writer),
        ("org.lab.Paint".to_string(), paint(kind, 7)),
        ("lab-notes".to_string(), notes),
    ]
    .into_iter()
    .collect()
}

fn target(kind: Kind) -> Target {
    Target {
        system: kind.apps(),
        arch: AppArch {
            known: true,
            x86: false,
            x64: true,
            arm64: false,
        },
        linux: (kind == Kind::Linux).then_some(LinuxPackages::Apt),
    }
}

fn no_winget(_: &FoundApp) -> Vec<(String, WingetBy)> {
    vec![]
}

struct NewPerson {
    kind: Kind,
    rules: LandingRules,
    extras: Option<Vec<u8>>,
    installed: AppList,
    folders: Vec<(FolderRole, String)>,
    plan: Option<FullPlan>,
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
            system: self.kind.settings(),
            target: target(self.kind),
            installed: &self.installed,
            folders: self.folders.clone(),
            winget: &no_winget,
        };
        self.plan = Some(FullPlan::build(
            record,
            MOVER,
            Some((&x, digest)),
            &here,
            &mut |_, _| Some(None),
        ));
        Ok(())
    }
    fn landing(&self, item: &Item) -> Option<LandingPlace> {
        self.rules.landing(item)
    }
    fn extras(&mut self, bytes: Vec<u8>) {
        self.extras = Some(bytes);
    }
}

fn put(p: &Path, bytes: &[u8]) {
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, bytes).unwrap();
}

/// The setting `key` of the lab app, as the stand-in holds it (text as text, decoded).
fn lab_setting(kind: Kind, t: &SettingsTools, key: &str) -> Option<String> {
    let s = t.0.borrow();
    match kind {
        Kind::Linux => s
            .dconf
            .get(&format!("/org/fakevendor/labwriter/{key}"))
            .map(|v| match gvariant::parse(v) {
                Ok(gvariant::GValue::Str(t)) => t,
                _ => v.clone(),
            }),
        Kind::Mac => s
            .prefs
            .get("org.fakevendor.labwriter")
            .and_then(|d| d.iter().find(|(k, _)| k == key))
            .map(|(_, v)| match v {
                Plist::String(t) => t.clone(),
                v => format!("{v:?}"),
            }),
    }
}

async fn full_move(kind: Kind) {
    let old = tempfile::tempdir().unwrap();
    let new = tempfile::tempdir().unwrap();
    let jdir = tempfile::tempdir().unwrap();

    // The old laptop's files. Each is read from a source of its own (a Mac's or Windows' own
    // drive cannot hold the case twins side by side); the record names them as the old Linux
    // laptop did.
    let mut items = Vec::new();
    let mut sources: BTreeMap<ItemId, PathBuf> = BTreeMap::new();
    for (n, (name, bytes)) in FILES.iter().enumerate() {
        let i = file_item(FolderRole::Documents, name, bytes.len() as u64);
        let src = old.path().join("sources").join(format!("{n}"));
        put(&src, bytes);
        sources.insert(i.id, src);
        items.push(i);
    }
    let mut record = Record::new(laptop());
    record.items = items;
    record.apps.found = old_apps(kind);
    record.validate().unwrap();
    let old_docs = docs(old.path());
    let new_docs = docs(new.path());
    std::fs::create_dir_all(&old_docs).unwrap();

    // The old laptop's settings: the lab app's, one of them a path to a moved file, one a secret.
    let old_tools = SettingsTools::default();
    {
        let mut s = old_tools.0.borrow_mut();
        let last = old_docs.join("letter.txt").display().to_string();
        match kind {
            Kind::Linux => {
                let k = |n: &str| format!("/org/fakevendor/labwriter/{n}");
                s.dconf.insert(k("theme"), "'ocean'".into());
                s.dconf.insert(k("zoom"), "1.25".into());
                s.dconf.insert(k("last-file"), gvariant::quote(&last));
                s.dconf.insert(k("api-key"), "'abc'".into());
            }
            Kind::Mac => {
                s.prefs.insert(
                    "org.fakevendor.labwriter".into(),
                    vec![
                        ("Theme".into(), Plist::String("ocean".into())),
                        ("Zoom".into(), Plist::Real(1.5)),
                        ("LastFile".into(), Plist::String(last)),
                        ("AuthToken".into(), Plist::String("abc".into())),
                    ],
                );
            }
        }
    }
    let mut old_mech = mech(kind.settings(), old_tools.clone());
    let discovered = match kind {
        Kind::Linux => discover_dconf(&mut old_mech),
        Kind::Mac => discover_mac(&mut old_mech),
    };

    // The old laptop's app data, where this person's own settings put it, and what the system
    // runs at sign-in (T26-026).
    let (data_root, startups): (PathBuf, Vec<PathBuf>) = match kind {
        Kind::Linux => {
            // This person keeps their settings elsewhere (their own systemd environment).
            put(
                &old.path().join(".config/environment.d/50-xdg.conf"),
                b"XDG_CONFIG_HOME=$HOME/settings\n",
            );
            let conf = old.path().join("settings");
            put(
                &conf.join("autostart").join("labmail.desktop"),
                b"[Desktop Entry]\nType=Application\nExec=labmail --background\n",
            );
            put(
                &conf.join("systemd").join("user").join("labmail.service"),
                b"[Service]\nExecStart=/usr/bin/labmail\n",
            );
            let startups = vec![conf.join("autostart"), conf.join("systemd")];
            (conf, startups)
        }
        Kind::Mac => (
            old.path().join("Library").join("Application Support"),
            vec![],
        ),
    };
    put(
        &data_root.join("LabMail").join("folders.json"),
        br#"{"inbox": 12, "sent": 3}"#,
    );
    put(
        &data_root.join("LabMail").join("signature.txt"),
        b"Ada, from the lab",
    );
    let mut extras = from_discovery(
        kind.settings(),
        discovered,
        vec![OldFolder {
            role: FolderRole::Documents,
            path: old_docs.display().to_string(),
        }],
    );
    let found =
        pctwin_fullmove::extras::SpecialScan::start_for(kind.special(), &mover(), old.path())
            .unwrap()
            .wait(&|| false)
            .expect("2.5 answered in time");
    assert!(
        found.lines.iter().any(|l| l.name.contains("LabMail")),
        "the app data is found where this person's own settings put it: {found:#?}"
    );
    for p in &startups {
        assert!(
            found.planned.iter().all(|c| !c.base.starts_with(p)),
            "nothing the system runs at sign-in is planned to move: {p:?}"
        );
    }
    assert!(found.not_moved >= startups.len() as u64);
    extras.with_special(Some(&found));
    let extras_bytes = extras.encode();

    // The new laptop.
    let mut table = Destinations::new();
    std::fs::create_dir_all(&new_docs).unwrap();
    table
        .approve(
            role_label(FolderRole::Documents),
            Approved::MyFolders,
            &new_docs,
        )
        .unwrap();
    let journal = Journal::open_with_record_key(
        &jdir.path().join("journal.redb"),
        pctwin_journal::keystore::TagKey::from_bytes([0x5a; 32]),
    )
    .unwrap();
    let rules = LandingRules {
        mover: MOVER.into(),
        roles: [(FolderRole::Documents, Storage::SystemDrive)]
            .into_iter()
            .collect(),
        fallback: Default::default(),
        held: Default::default(),
        left_out: Default::default(),
        chosen: Default::default(),
    };
    let mut person = NewPerson {
        kind,
        rules: rules.clone(),
        extras: None,
        installed: AppList::default(),
        folders: vec![(FolderRole::Documents, new_docs.display().to_string())],
        plan: None,
    };

    let (mut a, mut b) = mem_pair();
    let (old_live, new_live) = (Live::default(), Live::default());
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
                |i| sources.get(&i.id).cloned(),
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

    // 1. Files (T26-025): every one landed, byte for byte, under a name of its own on this drive.
    let mut landed_names: Vec<String> = Vec::new();
    for (name, bytes) in FILES {
        let item = received
            .record
            .items
            .iter()
            .find(|i| {
                i.path
                    .parts()
                    .last()
                    .is_some_and(|p| p.on_disk() == std::ffi::OsStr::new(name))
            })
            .unwrap();
        let f = received.files.iter().find(|f| f.item == item.id).unwrap();
        let Some(ReceiveOutcome::Finished(d)) = &f.outcome else {
            panic!("{name:?} did not land: {f:?}");
        };
        let at = new_docs.join(&d.final_path);
        assert_eq!(std::fs::read(&at).unwrap(), bytes, "{name:?} at {at:?}");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&at).unwrap().permissions().mode();
            assert_eq!(mode & 0o111, 0, "{name:?} never lands able to run");
        }
        landed_names.push(d.final_path.clone());
    }
    // Each landed at a name of its own (each read back above with its own bytes, so no two
    // share one on this drive, whatever it does with case and accent forms).
    let distinct: std::collections::BTreeSet<&String> = landed_names.iter().collect();
    assert_eq!(distinct.len(), FILES.len(), "{landed_names:?}");
    // The old laptop's own copies are as they were.
    for (id, src) in &sources {
        let n = record.items.iter().position(|i| i.id == *id).unwrap();
        assert_eq!(std::fs::read(src).unwrap(), FILES[n].1);
    }

    // The plan.
    let plan = person
        .plan
        .take()
        .expect("the plan was shown at the approval");

    // 2-3. Apps and settings, journaled.
    let new_tools = SettingsTools::default();
    let mut store = Store::new(
        Catalogue::starter().unwrap(),
        mech(kind.settings(), new_tools.clone()),
    );
    store.remember(plan.settings.book()).unwrap();
    let mut adapter = SettingsAdapter {
        store,
        platform: Platform {
            os: kind.os(),
            arch: Arch::X86_64,
        },
    };
    let stand_in = PackageStandIn {
        after: Rc::new(after_install(kind)),
        ..Default::default()
    };
    let listed = stand_in.list.clone();
    let list = || AppList {
        found: listed.borrow().clone(),
        not_read: vec![],
    };
    let key = |remote: &str, _: bool| (remote == "flathub").then_some([7u8; 32]);
    let mut linux = LinuxInstaller {
        runner: &stand_in,
        tools: PackageTools {
            pkcon: Some("/usr/bin/pkcon".into()),
            flatpak: Some("/usr/bin/flatpak".into()),
            snap: Some("/usr/bin/snap".into()),
        },
        home: new.path().to_path_buf(),
        session: vec![],
        flatpak_key: &key,
        list: &list,
    };
    let mut mac = NoInstaller {
        why: "this laptop has no way for PCTwin to install apps yet",
    };
    let installer: &mut dyn Installer = match kind {
        Kind::Linux => &mut linux,
        Kind::Mac => &mut mac,
    };
    let rebuild_rec = Rebuild::new(&journal, &keys().install, &keys().setting);
    let scope = Scope::for_a_move().unwrap();
    let actor = Actor {
        acting_account: HERE.into(),
        for_account: HERE.into(),
        permission: Permission::OwnFolders,
    };
    let files = pctwin_fullmove::run::files_landed(
        &plan,
        received.files.iter().filter_map(|f| {
            let item = received.record.items.iter().find(|i| i.id == f.item)?;
            match &f.outcome {
                Some(ReceiveOutcome::Finished(d)) => Some((item, new_docs.join(&d.final_path))),
                _ => None,
            }
        }),
    );
    let done = rebuild(
        &plan,
        &pctwin_fullmove::run::DataDone {
            files,
            ..Default::default()
        },
        installer,
        &mut adapter,
        &rebuild_rec,
        scope,
        &actor,
        LaptopId::from_hex("ffeeddccbbaa99887766554433221100").unwrap(),
        &mut |_| {},
    );
    assert_eq!(done.stopped, None);
    let result = |name: &str| {
        done.apps
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, r)| r.result)
            .unwrap_or_else(|| panic!("{name} has a result: {:#?}", done.apps))
    };
    match kind {
        Kind::Linux => {
            assert_eq!(result("lab-writer").status(), ItemStatus::Reinstalled);
            assert_eq!(result("Lab Paint").status(), ItemStatus::Reinstalled);
            assert_eq!(
                result("lab-notes").reason(),
                Some(Reason::FinishByHand),
                "installed from another publisher: the person looks at it"
            );
            assert_eq!(result("lab-old-tool").reason(), Some(Reason::ManualInstall));
            assert_eq!(result("lab-fedora").reason(), Some(Reason::ManualInstall));
            let calls = stand_in.calls.borrow();
            let pkcon: Vec<_> = calls.iter().filter(|c| c[0].ends_with("pkcon")).collect();
            assert_eq!(
                pkcon,
                [&vec![
                    "/usr/bin/pkcon".to_string(),
                    "--noninteractive".into(),
                    "install".into(),
                    "lab-writer".into()
                ]],
                "every package in one transaction: one prompt"
            );
            assert!(
                calls.iter().any(|c| c.ends_with(&[
                    "--user".to_string(),
                    "flathub".into(),
                    "org.lab.Paint".into()
                ])),
                "the Flatpak from the remote with the same address and key, for this person: {calls:?}"
            );
            assert!(done.undo.apps.contains(&"lab-writer".to_string()));
            assert!(done.undo.apps.contains(&"lab-notes".to_string()));
            assert_eq!(done.restore_point, Some(RestorePoint::NotWindows));
        }
        Kind::Mac => {
            assert_eq!(result("Lab Writer").reason(), Some(Reason::ManualInstall));
        }
    }
    for (label, _, r) in &done.settings {
        assert_eq!(r.status(), ItemStatus::Converted, "{label}: {r:?}");
    }
    assert!(!done.settings.is_empty());
    let (theme, zoom, last, secret) = match kind {
        Kind::Linux => ("theme", "zoom", "last-file", "api-key"),
        Kind::Mac => ("Theme", "Zoom", "LastFile", "AuthToken"),
    };
    assert!(lab_setting(kind, &new_tools, theme).is_some_and(|v| v.contains("ocean")));
    assert!(lab_setting(kind, &new_tools, zoom).is_some());
    let moved_letter = new_docs.join("letter.txt").display().to_string();
    // Only where the paths are this system's own (a Windows runner plays both laptops with
    // Windows paths, which no Linux laptop or Mac has).
    assert!(
        cfg!(windows)
            || lab_setting(kind, &new_tools, last).is_some_and(|v| v.contains(&moved_letter)),
        "a path into a moved folder points at the moved file: {:?}",
        lab_setting(kind, &new_tools, last)
    );
    assert_eq!(
        lab_setting(kind, &new_tools, secret),
        None,
        "a secret never moves"
    );
    for s in &done.special {
        assert_ne!(
            s.result.status(),
            ItemStatus::Copied,
            "nothing claimed unproven"
        );
    }

    // 5. The person changes the theme afterwards; undo keeps theirs and puts the rest back.
    {
        let mut s = new_tools.0.borrow_mut();
        match kind {
            Kind::Linux => {
                s.dconf
                    .insert("/org/fakevendor/labwriter/theme".into(), "'forest'".into());
            }
            Kind::Mac => {
                let d = s.prefs.get_mut("org.fakevendor.labwriter").unwrap();
                d.retain(|(k, _)| k != "Theme");
                d.push(("Theme".into(), Plist::String("forest".into())));
            }
        }
    }
    let lines = undo::undo(&rebuild_rec, &done.undo, &mut adapter, None).unwrap();
    assert!(
        lines
            .iter()
            .any(|l| matches!(l.result, undo::Result::KeptChanged { .. })),
        "{lines:#?}"
    );
    assert!(lab_setting(kind, &new_tools, theme).is_some_and(|v| v.contains("forest")));
    assert_eq!(
        lab_setting(kind, &new_tools, zoom),
        None,
        "put back: not set before"
    );
    // The old laptop's settings are as they were.
    assert!(lab_setting(kind, &old_tools, theme).is_some_and(|v| v.contains("ocean")));
    // Installed apps are the person's to remove, said as such.
    if kind == Kind::Linux {
        assert!(
            lines.iter().any(|l| l.name.contains("lab-writer")),
            "{lines:#?}"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn linux_to_linux_moves_files_apps_settings_and_undo_honestly() {
    full_move(Kind::Linux).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mac_to_mac_moves_files_settings_and_says_what_the_person_installs() {
    full_move(Kind::Mac).await;
}
