//! The full move between two DIFFERENT systems (Phase 2 exit gate: "every supported direction
//! passes its scan, transfer, rebuild and undo fixtures"), for the six cross-system directions
//! among Windows, Mac and Linux (ChromeOS and Windows S mode are not supported, decided 30
//! October 2026). Windows to Windows is `tests/lab.rs`; Linux to Linux and Mac to Mac are
//! `tests/lab_unix.rs`.
//!
//! Both laptops are in one test over an in-memory link, through the same engine the app runs.
//! The old laptop is a stand-in of system A: its files, its apps as A's own lists name them, its
//! settings read with A's own tools (stand-ins of `defaults` and `dconf`, or a registry hive in
//! memory), and its app data laid out where A keeps it, found by 2.5 for A. The new laptop is a
//! stand-in of system B: B's settings tools, B's installer (a stand-in that checks the maker as
//! B's real chain does; nothing is installed), B's layout for app data. The files land on the
//! file system of the machine the test runs on: every direction runs on every CI runner (the
//! `rust` job runs the whole workspace on Windows, Mac and Linux), so each direction lands its
//! files on B's own file system on B's runner, where the name checks for B bite.
//!
//! Per direction:
//!
//! 1. **Scan**: the old laptop's record, settings discovery and 2.5 app data (VS Code's folder,
//!    which the 2.5 catalogue places on every system) are made with A's stand-ins.
//! 2. **Transfer**: every file lands byte for byte under a name of its own that B can hold; the
//!    old laptop's files are unchanged.
//! 3. **Rebuild**: an app is reinstalled only as the same maker's edition for B that the
//!    `app-counterparts` catalogue names, through B's installer stand-in, which checks the maker;
//!    otherwise it ends needs-action with the choices the catalogue gives (the maker's web
//!    version). Settings cross only as the settings catalogue maps them; an app's own settings
//!    never cross between systems. App data lands placed for B, its settings files put right for
//!    B (VS Code's shortcuts, Ctrl and Cmd).
//! 4. **Undo** puts the settings back and keeps one the person changed; apps PCTwin installed are
//!    listed for the person to remove.
//!
//! What the engine does not do yet is written as a test too, marked `#[ignore]` with the reason:
//! undoing landed app data on a Mac or Linux laptop
//! (`undo_removes_landed_app_data_on_mac_and_linux`).
// Tests make files to test on.
#![allow(clippy::disallowed_methods)]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use pctwin_apps::after::{self, InstallScope, Installed, Opened};
use pctwin_apps::found::windows::{RawEntry, RegValue as RawValue};
use pctwin_apps::routes::brew::CaskIndex;
use pctwin_apps::routes::counterpart::ChoiceKind;
use pctwin_apps::routes::{LinuxPackages, Route, Target, WingetBy};
use pctwin_fullmove::data::{Landing, Sink, Source, land_all};
use pctwin_fullmove::extras::{Extras, OldFolder, SpecialScan, from_discovery};
use pctwin_fullmove::plan::{AppEntry, FullPlan, NewLaptop, Title};
use pctwin_fullmove::run::{AppResult, DataDone, Installer, rebuild};
use pctwin_fullmove::undo;
use pctwin_fullmove::unix::{LinuxInstaller, Tools as PackageTools};
use pctwin_gate::{Approved, Destination, Destinations};
use pctwin_journal::{Actor, Journal, Permission, Rebuild, RestorePoint, Scope};
use pctwin_maker::run::{Call, Ran, RunError, Runner as ToolRunner};
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
use pctwin_settings::classify::Why;
use pctwin_settings::found::{discover_dconf, discover_mac, discover_registry};
use pctwin_settings::gvariant;
use pctwin_settings::mechanism::Mechanisms;
use pctwin_settings::plist::{self, Plist};
use pctwin_settings::registry::{Hive, MemHive, RegValue};
use pctwin_settings::runner::{Output, Runner, Tool};
use pctwin_settings::{SettingsAdapter, Store};
use pctwin_special::capture::{Holders, Holding};
use pctwin_transfer::{Channel, ChannelError, LandingPlace, ReceiveOutcome, role_label};
use pctwin_types::{ItemStatus, Reason};
use tokio::sync::mpsc;

const MOVER: &str = "1000";
const HERE: &str = "1001";

/* ---------- The link ---------- */

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

/* ---------- The systems ---------- */

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Windows,
    Mac,
    Linux,
}

impl Kind {
    fn settings(self) -> System {
        match self {
            Kind::Windows => System::Windows,
            Kind::Mac => System::Mac,
            Kind::Linux => System::Gnome,
        }
    }
    fn special(self) -> pctwin_special::roots::System {
        match self {
            Kind::Windows => pctwin_special::roots::System::Windows,
            Kind::Mac => pctwin_special::roots::System::Mac,
            Kind::Linux => pctwin_special::roots::System::Linux,
        }
    }
    fn apps(self) -> AppSystem {
        match self {
            Kind::Windows => AppSystem::Windows,
            Kind::Mac => AppSystem::Mac,
            Kind::Linux => AppSystem::Linux,
        }
    }
    fn os(self) -> Os {
        match self {
            Kind::Windows => Os::Windows,
            Kind::Mac => Os::Macos,
            Kind::Linux => Os::Linux,
        }
    }
    fn gate(self) -> pctwin_gate::Platform {
        match self {
            Kind::Windows => pctwin_gate::Platform::Windows,
            Kind::Mac => pctwin_gate::Platform::MacOs,
            Kind::Linux => pctwin_gate::Platform::Linux,
        }
    }
    /// Whether the machine running the test is this system (its file system is B's own).
    fn is_host(self) -> bool {
        match self {
            Kind::Windows => cfg!(windows),
            Kind::Mac => cfg!(target_os = "macos"),
            Kind::Linux => cfg!(target_os = "linux"),
        }
    }
    /// The key that goes with a shortcut on this system (VS Code's `ctrl` or `cmd`).
    fn shortcut(self) -> &'static str {
        match self {
            Kind::Mac => "cmd+s",
            _ => "ctrl+s",
        }
    }
    /// Where VS Code keeps its settings folder under a home on this system.
    fn code_folder(self, home: &Path) -> PathBuf {
        match self {
            Kind::Windows => home.join("AppData").join("Roaming").join("Code"),
            Kind::Mac => home
                .join("Library")
                .join("Application Support")
                .join("Code"),
            Kind::Linux => home.join(".config").join("Code"),
        }
    }
}

/* ---------- Settings stores in memory: `defaults`, `dconf`, `gsettings` ---------- */

#[derive(Default)]
struct Prefs {
    /// domain -> key -> value
    prefs: BTreeMap<String, Vec<(String, Plist)>>,
    /// full dconf key -> GVariant text
    dconf: BTreeMap<String, String>,
    /// What `gsettings get` says for a key the person never set (the schema's default).
    schema_defaults: BTreeMap<String, String>,
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

/// A gsettings schema and key as the dconf path GNOME keeps it at.
fn dconf_path(schema: &str, key: &str) -> String {
    format!("/{}/{key}", schema.replace('.', "/"))
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
            (Tool::Gsettings, ["get", schema, key]) => {
                let p = dconf_path(schema, key);
                match s.dconf.get(&p).or_else(|| s.schema_defaults.get(&p)) {
                    Some(v) => ok(&format!("{v}\n")),
                    None => fail(),
                }
            }
            (Tool::Gsettings, ["set", schema, key, v]) => {
                gvariant::parse(v)?;
                s.dconf.insert(dconf_path(schema, key), v.to_string());
                ok("")
            }
            (Tool::Gsettings, ["reset", schema, key]) => {
                s.dconf.remove(&dconf_path(schema, key));
                ok("")
            }
            (Tool::Gsettings, ["writable", ..]) => ok("true"),
            _ => fail(),
        }
    }
}

/* ---------- The installers of the new laptop (nothing is installed) ---------- */

/// Windows' install chain as a stand-in: the winget package's download is signed by the maker
/// this table names; like the helper, it is installed only when that is the maker the plan
/// names for the app; then Windows' own list is read again and the real verdict
/// ([`after::probe`], [`after::result`]) decides.
#[derive(Default)]
struct WindowsStandIn {
    asked: Vec<String>,
    entries: Vec<RawEntry>,
}

/// What each winget package's download is signed by (organisation, country).
fn winget_signer(id: &str) -> Option<(&'static str, &'static str)> {
    match id {
        "Mozilla.Firefox" => Some(("Mozilla Corporation", "US")),
        _ => None,
    }
}

impl Installer for WindowsStandIn {
    fn restore_point(&mut self) -> RestorePoint {
        RestorePoint::NotMade {
            why: "the test lab makes no restore point".into(),
        }
    }
    fn install(&mut self, apps: &[&AppEntry], done: &mut dyn FnMut()) -> Vec<AppResult> {
        apps.iter()
            .map(|e| {
                done();
                let s = e.subject();
                assert_eq!(s.system, AppSystem::Windows, "only a Windows app here");
                let id = e.winget().expect("a Windows install is from winget");
                self.asked.push(id.to_string());
                let signed_by = winget_signer(id);
                let named = match &s.maker {
                    AppMaker::Windows(sig) => {
                        Some((sig.organization.as_deref(), sig.country.as_deref()))
                    }
                    _ => None,
                };
                if signed_by.is_none() || named != signed_by.map(|(o, c)| (Some(o), Some(c))) {
                    // The helper refuses a download by anyone else.
                    return AppResult::because(Reason::NotVerified);
                }
                let mut values = BTreeMap::new();
                values.insert("displayname".into(), RawValue::Text(s.name.clone()));
                if let Some(p) = &s.publisher {
                    values.insert("publisher".into(), RawValue::Text(p.clone()));
                }
                values.insert("displayversion".into(), RawValue::Text("150.0".into()));
                self.entries.push(RawEntry {
                    view: RegistryView::Machine,
                    key: s.name.clone(),
                    values,
                });
                let probe = after::probe(s, InstallScope::Machine, &self.entries);
                AppResult::installed(
                    after::result(
                        s,
                        Installed::Done {
                            restart_needed: false,
                        },
                        &probe,
                        &[],
                        Opened::NotChecked,
                    ),
                    vec![],
                )
            })
            .collect()
    }
}

/// The Mac's install chain as a stand-in: a cask's app is signed by the developer team this
/// table names, and is placed only when that is the team the plan names for the app (step 3 of
/// `pctwin_fullmove::mac`).
#[derive(Default)]
struct MacStandIn {
    asked: Vec<String>,
}

fn cask_team(token: &str) -> Option<&'static str> {
    match token {
        "firefox" => Some("43AQ936H96"),
        _ => None,
    }
}

impl Installer for MacStandIn {
    fn restore_point(&mut self) -> RestorePoint {
        RestorePoint::NotWindows
    }
    fn install(&mut self, apps: &[&AppEntry], done: &mut dyn FnMut()) -> Vec<AppResult> {
        apps.iter()
            .map(|e| {
                done();
                let s = e.subject();
                assert_eq!(s.system, AppSystem::Mac, "only a Mac app here");
                let token = e.plan.routes.iter().find_map(|r| match r {
                    Route::Cask { token, .. } => Some(token.clone()),
                    _ => None,
                });
                let Some(token) = token else {
                    return AppResult::because(Reason::ManualInstall);
                };
                self.asked.push(token.clone());
                let team = match &s.maker {
                    AppMaker::MacTeam { team_id } => Some(team_id.as_str()),
                    _ => None,
                };
                if team.is_none() || team != cask_team(&token) {
                    return AppResult::because(Reason::NotVerified);
                }
                AppResult::installed(
                    pctwin_types::ItemResult::done(ItemStatus::Reinstalled).unwrap(),
                    vec![],
                )
            })
            .collect()
    }
}

/// Linux's package tools as a stand-in: every call is recorded; none is expected (no Linux
/// edition in the shipped counterparts catalogue names the Flatpak remote it comes from).
#[derive(Default, Clone)]
struct PackageStandIn {
    calls: Rc<RefCell<Vec<String>>>,
}

impl ToolRunner for PackageStandIn {
    fn run(&self, call: &Call) -> Result<Ran, RunError> {
        self.calls
            .borrow_mut()
            .push(call.program.to_string_lossy().into_owned());
        Ok(Ran {
            code: Some(1),
            stdout: String::new(),
            stderr: "nothing is installed in the test lab".into(),
        })
    }
}

/* ---------- The old laptop ---------- */

fn laptop() -> LaptopId {
    LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap()
}

fn mover() -> Owner {
    Owner::Person {
        account_id: MOVER.into(),
    }
}

fn file_item(name: &str, size: u64) -> Item {
    let place = Place {
        role: FolderRole::Documents,
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

/// The old laptop's files: names its own system holds that another may not (letter case, the
/// characters and device names Windows forbids, a program file).
fn files_of(a: Kind) -> Vec<(&'static str, &'static [u8])> {
    match a {
        Kind::Linux => vec![
            ("letter.txt", b"Dear Ada"),
            ("Report.txt", b"upper case report"),
            ("report.txt", b"lower case report"),
            ("Notes: May?.txt", b"a name Windows cannot hold"),
            ("CON.txt", b"a device name on Windows"),
            ("run.sh", b"#!/bin/sh\necho hi\n"),
        ],
        Kind::Mac => vec![
            ("letter.txt", b"Dear Ada"),
            ("Budget: 2026.numbers", b"a colon, fine on a Mac"),
            ("caf\u{e9}.txt", b"an accent"),
            ("run.command", b"#!/bin/sh\necho hi\n"),
        ],
        Kind::Windows => vec![
            ("letter.txt", b"Dear Ada"),
            ("Holiday Photo.JPG", b"sand and sea"),
            ("run.cmd", b"@echo hi\r\n"),
        ],
    }
}

fn found(a: Kind, name: &str, found: FoundWhere, maker: AppMaker) -> FoundApp {
    FoundApp {
        name: name.into(),
        version: Some("2.0".into()),
        publisher: None,
        owner: Owner::Shared,
        system: a.apps(),
        found,
        arch: AppArch::UNKNOWN,
        maker,
        web: None,
        update_feed: None,
        installers: vec![],
        notes: vec![],
    }
}

fn win_signed(org: &str, country: &str) -> AppMaker {
    AppMaker::Windows(Box::new(SignerId {
        organization: Some(org.into()),
        country: Some(country.into()),
        certificate: [9; 32],
        ..Default::default()
    }))
}

fn win_entry(name: &str) -> FoundWhere {
    FoundWhere::WindowsEntry {
        view: RegistryView::Machine,
        key: name.into(),
        product_code: None,
    }
}

fn mac_bundle(id: &str, folder: &str) -> FoundWhere {
    FoundWhere::MacBundle {
        bundle_id: id.into(),
        folder: folder.into(),
        app_store_id: None,
    }
}

fn flatpak(id: &str) -> (FoundWhere, AppMaker) {
    (
        FoundWhere::Flatpak {
            app_id: id.into(),
            remote: "flathub".into(),
            remote_url: Some("https://dl.flathub.org/repo/".into()),
            system_wide: false,
        },
        AppMaker::Flatpak {
            remote_url: "https://dl.flathub.org/repo/".into(),
            app_id: id.into(),
            key: [7; 32],
        },
    )
}

/// The old laptop's apps, as its own system lists them: Firefox and Spotify (each an edition the
/// `app-counterparts` catalogue names) and an app no row names. Returns (the apps, Firefox's
/// name there, Spotify's name there, the unnamed app's name there).
fn apps_of(a: Kind) -> (Vec<FoundApp>, &'static str, &'static str, &'static str) {
    match a {
        Kind::Windows => (
            vec![
                found(
                    a,
                    "Mozilla Firefox (x64 en-US)",
                    win_entry("Mozilla Firefox (x64 en-US)"),
                    win_signed("Mozilla Corporation", "US"),
                ),
                found(
                    a,
                    "Spotify",
                    win_entry("Spotify"),
                    win_signed("Spotify AB", "SE"),
                ),
                found(
                    a,
                    "Lab Writer",
                    win_entry("Lab Writer"),
                    win_signed("PCTwin Lab Ltd", "GB"),
                ),
            ],
            "Mozilla Firefox (x64 en-US)",
            "Spotify",
            "Lab Writer",
        ),
        Kind::Mac => (
            vec![
                found(
                    a,
                    "Firefox",
                    mac_bundle("org.mozilla.firefox", "Firefox.app"),
                    AppMaker::MacTeam {
                        team_id: "43AQ936H96".into(),
                    },
                ),
                found(
                    a,
                    "Spotify",
                    mac_bundle("com.spotify.client", "Spotify.app"),
                    AppMaker::MacTeam {
                        team_id: "2FNC3A47ZF".into(),
                    },
                ),
                found(
                    a,
                    "Lab Writer",
                    mac_bundle("org.lab.writer", "Lab Writer.app"),
                    AppMaker::MacTeam {
                        team_id: "ABCDE12345".into(),
                    },
                ),
            ],
            "Firefox",
            "Spotify",
            "Lab Writer",
        ),
        Kind::Linux => {
            let (ff, ff_maker) = flatpak("org.mozilla.firefox");
            let (sp, sp_maker) = flatpak("com.spotify.Client");
            (
                vec![
                    found(a, "Firefox", ff, ff_maker),
                    found(a, "Spotify", sp, sp_maker),
                    found(
                        a,
                        "lab-writer",
                        FoundWhere::Deb {
                            package: "lab-writer".into(),
                        },
                        AppMaker::Repository {
                            origin: "http://archive.ubuntu.com/ubuntu noble/main".into(),
                        },
                    ),
                ],
                "Firefox",
                "Spotify",
                "lab-writer",
            )
        }
    }
}

/// The old laptop's settings: the theme dark and the pointer at its fastest (both rows of the
/// settings catalogue on every system), and an app's own setting no catalogue row names.
fn old_settings(a: Kind) -> (SettingsTools, MemHive) {
    let tools = SettingsTools::default();
    let mut hive = MemHive::default();
    match a {
        Kind::Windows => {
            let set = |h: &mut MemHive, k: &str, n: &str, v: RegValue| h.set(k, n, &v).unwrap();
            set(
                &mut hive,
                "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize",
                "AppsUseLightTheme",
                RegValue::Dword(0),
            );
            set(
                &mut hive,
                "Control Panel\\Mouse",
                "MouseSensitivity",
                RegValue::Sz("20".into()),
            );
            set(
                &mut hive,
                "Software\\PCTwinLabVendor\\Writer",
                "Theme",
                RegValue::Sz("ocean".into()),
            );
        }
        Kind::Mac => {
            let mut s = tools.0.borrow_mut();
            s.prefs.insert(
                "NSGlobalDomain".into(),
                vec![
                    ("AppleInterfaceStyle".into(), Plist::String("Dark".into())),
                    ("com.apple.mouse.scaling".into(), Plist::Real(3.0)),
                ],
            );
            s.prefs.insert(
                "org.fakevendor.labwriter".into(),
                vec![("Theme".into(), Plist::String("ocean".into()))],
            );
        }
        Kind::Linux => {
            let mut s = tools.0.borrow_mut();
            s.dconf.insert(
                "/org/gnome/desktop/interface/color-scheme".into(),
                "'prefer-dark'".into(),
            );
            s.dconf.insert(
                "/org/gnome/desktop/peripherals/mouse/speed".into(),
                "1.0".into(),
            );
            s.dconf
                .insert("/org/fakevendor/labwriter/theme".into(), "'ocean'".into());
        }
    }
    (tools, hive)
}

/// The new laptop's settings before the move: the theme light, the pointer in the middle.
fn new_settings(b: Kind) -> (SettingsTools, Option<MemHive>) {
    let tools = SettingsTools::default();
    match b {
        Kind::Windows => {
            let mut hive = MemHive::default();
            hive.set(
                "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize",
                "AppsUseLightTheme",
                &RegValue::Dword(1),
            )
            .unwrap();
            hive.set(
                "Control Panel\\Mouse",
                "MouseSensitivity",
                &RegValue::Sz("10".into()),
            )
            .unwrap();
            (tools, Some(hive))
        }
        Kind::Mac => {
            tools.0.borrow_mut().prefs.insert(
                "NSGlobalDomain".into(),
                vec![("com.apple.mouse.scaling".into(), Plist::Real(1.0))],
            );
            (tools, None)
        }
        Kind::Linux => {
            {
                let mut s = tools.0.borrow_mut();
                s.schema_defaults.insert(
                    "/org/gnome/desktop/interface/color-scheme".into(),
                    "'default'".into(),
                );
                s.schema_defaults.insert(
                    "/org/gnome/desktop/peripherals/mouse/speed".into(),
                    "0.0".into(),
                );
            }
            (tools, None)
        }
    }
}

/// The person changes the pointer speed on the new laptop after the move.
fn person_changes_speed(b: Kind, adapter: &mut SettingsAdapter, tools: &SettingsTools) {
    match b {
        Kind::Windows => adapter
            .store
            .mechanisms()
            .user
            .as_deref_mut()
            .unwrap()
            .set(
                "Control Panel\\Mouse",
                "MouseSensitivity",
                &RegValue::Sz("5".into()),
            )
            .unwrap(),
        Kind::Mac => {
            let mut s = tools.0.borrow_mut();
            let d = s.prefs.entry("NSGlobalDomain".into()).or_default();
            d.retain(|(k, _)| k != "com.apple.mouse.scaling");
            d.push(("com.apple.mouse.scaling".into(), Plist::Real(0.5)));
        }
        Kind::Linux => {
            tools.0.borrow_mut().dconf.insert(
                "/org/gnome/desktop/peripherals/mouse/speed".into(),
                "-0.5".into(),
            );
        }
    }
}

/// A catalogue row's value on the new laptop now, as the system shows it to the person (read
/// through the row's own place there and decoded, as the executor and undo read it). Undo's
/// promise is this value: putting the theme back to light on a Mac may leave the "switch
/// automatically" choice written as off where it was not set, which means the same.
fn read_row(
    adapter: &mut SettingsAdapter,
    b: Kind,
    id: &str,
) -> Option<pctwin_recipe::SettingValue> {
    let cat = Catalogue::starter().unwrap();
    let row = cat
        .settings
        .iter()
        .find(|r| r.id.as_str() == id)
        .unwrap_or_else(|| panic!("the catalogue has {id}"));
    assert!(
        row.places.contains_key(&b.settings()),
        "{id} has a place on {b:?}"
    );
    adapter.store.read(id).unwrap()
}

/// Whether the lab app's own setting is on the new laptop.
fn lab_setting_here(b: Kind, adapter: &mut SettingsAdapter, tools: &SettingsTools) -> bool {
    match b {
        Kind::Windows => adapter
            .store
            .mechanisms()
            .user
            .as_deref()
            .unwrap()
            .get("Software\\PCTwinLabVendor\\Writer", "Theme")
            .unwrap()
            .is_some(),
        Kind::Mac => tools
            .0
            .borrow()
            .prefs
            .contains_key("org.fakevendor.labwriter"),
        Kind::Linux => tools
            .0
            .borrow()
            .dconf
            .keys()
            .any(|k| k.contains("labwriter")),
    }
}

/// VS Code's settings folder on the old laptop: its keyboard shortcuts written for A.
fn put_code(a: Kind, home: &Path) {
    let code = a.code_folder(home).join("User");
    put(
        &code.join("keybindings.json"),
        format!(
            r#"[{{ "key": "{}", "command": "workbench.action.files.save" }}]"#,
            a.shortcut()
        )
        .as_bytes(),
    );
    put(
        &code.join("settings.json"),
        br#"{ "editor.fontSize": 15, "workbench.colorTheme": "Default Dark Modern" }"#,
    );
}

fn put(p: &Path, bytes: &[u8]) {
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, bytes).unwrap();
}

/* ---------- The new laptop's person ---------- */

struct Free;
impl Holders for Free {
    fn holding(&self, _: &[PathBuf]) -> Holding {
        Holding::Free
    }
}

struct AppOwnerOnly;
impl pctwin_special::land::OwnerOnly for AppOwnerOnly {
    fn private_folder(&self, path: &Path) -> std::io::Result<()> {
        #[cfg(windows)]
        return pctwin_helper_os::private_folder(path)
            .map_err(|e| std::io::Error::other(format!("{e:?}")));
        #[cfg(unix)]
        return pctwin_special::land::UnixOwnerOnly.private_folder(path);
    }
}

fn target(b: Kind) -> Target {
    Target {
        system: b.apps(),
        arch: AppArch {
            known: true,
            x86: false,
            x64: true,
            arm64: false,
        },
        linux: (b == Kind::Linux).then_some(LinuxPackages::Apt),
    }
}

/// winget, searched as the person on a Windows laptop: the package of the app's exact name.
fn winget(a: &FoundApp) -> Vec<(String, WingetBy)> {
    if a.name == "Mozilla Firefox (x64 en-US)" {
        vec![("Mozilla.Firefox".into(), WingetBy::Name)]
    } else {
        vec![]
    }
}

fn no_winget(_: &FoundApp) -> Vec<(String, WingetBy)> {
    vec![]
}

/// Homebrew's cask list as a Mac reads it: Firefox's cask.
fn casks() -> CaskIndex {
    CaskIndex::parse(
        br#"[{"token":"firefox","version":"150.0","sha256":"1d7aa96b51fc9110185047de18228ac87238fba5e42bba53c9d696f88878cba6",
        "url":"https://download-installer.cdn.mozilla.net/pub/firefox/releases/150.0/mac/en-US/Firefox%20150.0.dmg",
        "auto_updates":true,"deprecated":false,"disabled":false,"artifacts":[{"app":["Firefox.app"]}]}]"#,
    )
    .unwrap()
}

struct NewPerson {
    b: Kind,
    rules: LandingRules,
    extras: Option<Vec<u8>>,
    installed: AppList,
    folders: Vec<(FolderRole, String)>,
    casks: Option<CaskIndex>,
    /// What each catalogue row holds on this laptop now (`None`: it could not be read).
    now: BTreeMap<String, Option<Option<pctwin_recipe::SettingValue>>>,
    plan: Option<FullPlan>,
    sink: Option<Sink>,
    staging: PathBuf,
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
        let here = NewLaptop {
            system: self.b.settings(),
            target: target(self.b),
            installed: &self.installed,
            folders: self.folders.clone(),
            winget: if self.b == Kind::Windows {
                &winget
            } else {
                &no_winget
            },
            casks: self.casks.as_ref(),
        };
        let plan = FullPlan::build(
            record,
            MOVER,
            Some((&x, pctwin_fullmove::extras::digest(&bytes))),
            &here,
            &mut |_, key| self.now.get(key).cloned().unwrap_or(Some(None)),
        );
        // App data that copies, as proposed; no secret is ticked.
        self.sink = Some(Sink::new(plan.choice(&[]), self.staging.clone()));
        self.plan = Some(plan);
        Ok(())
    }
    fn landing(&self, item: &Item) -> Option<LandingPlace> {
        self.rules.landing(item)
    }
    fn extras(&mut self, bytes: Vec<u8>) {
        self.extras = Some(bytes);
    }
    fn data_in(&mut self) -> Option<&mut dyn pctwin_move::extras::DataIn> {
        self.sink
            .as_mut()
            .map(|s| s as &mut dyn pctwin_move::extras::DataIn)
    }
}

/* ---------- The move ---------- */

/// What a direction is held to: everything ([`Strict::all`]), or one part of it (the tests named
/// for that part).
#[derive(Clone, Copy, Default)]
struct Strict {
    /// The settings catalogue's rows cross, and undo puts them back keeping the person's edit.
    settings: bool,
    /// App data lands at B's place for the app, its settings files put right for B, and undo
    /// removes it.
    data: bool,
    /// On a Mac, the same maker's edition is installed from its cask.
    mac_installs: bool,
    /// Undo removes landed app data on a Mac or Linux laptop too.
    unix_data_undo: bool,
}

impl Strict {
    /// Everything this direction is held to.
    fn all(b: Kind) -> Self {
        Strict {
            settings: true,
            data: true,
            mac_installs: b == Kind::Mac,
            unix_data_undo: false,
        }
    }
}

async fn cross_move(a: Kind, b: Kind, strict: Strict) {
    assert_ne!(
        a, b,
        "the same-system directions are lab.rs and lab_unix.rs"
    );
    pctwin_parse::testing::install_readers();
    let t = tempfile::tempdir().unwrap();
    let old = t.path().join("old");
    let new = t.path().join("new");
    let staging = t.path().join("staging");
    let work = t.path().join("old-work");
    std::fs::create_dir_all(&staging).unwrap();

    /* 1. Scan, on the old laptop (system A), only read. */
    let files = files_of(a);
    let mut items = Vec::new();
    let mut sources: BTreeMap<ItemId, PathBuf> = BTreeMap::new();
    for (n, (name, bytes)) in files.iter().enumerate() {
        let i = file_item(name, bytes.len() as u64);
        // Each from a source of its own: the machine running the test may not hold A's names.
        let src = old.join("sources").join(format!("{n}"));
        put(&src, bytes);
        sources.insert(i.id, src);
        items.push(i);
    }
    let (apps, firefox, spotify, unnamed) = apps_of(a);
    let mut record = Record::new(laptop());
    record.items = items;
    record.apps.found = apps;
    record.validate().unwrap();
    let old_docs = old.join("Documents");
    std::fs::create_dir_all(&old_docs).unwrap();

    let (old_tools, old_hive) = old_settings(a);
    let discovered = match a {
        Kind::Windows => discover_registry(&old_hive),
        Kind::Mac => discover_mac(&mut Mechanisms {
            system: System::Mac,
            user: None,
            machine: None,
            runner: Some(Box::new(old_tools.clone())),
            localtime: None,
        }),
        Kind::Linux => discover_dconf(&mut Mechanisms {
            system: System::Gnome,
            user: None,
            machine: None,
            runner: Some(Box::new(old_tools.clone())),
            localtime: None,
        }),
    };
    put_code(a, &old);
    let code_before = std::fs::read(a.code_folder(&old).join("User/keybindings.json")).unwrap();
    let mut extras = from_discovery(
        a.settings(),
        discovered,
        vec![OldFolder {
            role: FolderRole::Documents,
            path: old_docs.display().to_string(),
        }],
    );
    let special = SpecialScan::start_for(a.special(), &mover(), &old)
        .unwrap()
        .wait(&|| false)
        .expect("2.5 answered in time");
    assert!(
        special.lines.iter().any(|l| l.name.contains("Code")),
        "2.5 finds VS Code's folder where {a:?} keeps it: {:#?}",
        special.lines
    );
    extras.with_special(Some(&special));
    // The settings catalogue's rows, read only with A's own tools (move protocol 9).
    let mut old_store = Store::new(
        Catalogue::starter().unwrap(),
        Mechanisms {
            system: a.settings(),
            user: (a == Kind::Windows).then(|| Box::new(old_hive.clone()) as Box<dyn Hive>),
            machine: None,
            runner: (a != Kind::Windows).then(|| Box::new(old_tools.clone()) as Box<dyn Runner>),
            localtime: None,
        },
    );
    extras.with_catalogue(old_store.capture());
    let extras_bytes = extras.encode();
    let apps_by_id: BTreeMap<String, String> = special
        .lines
        .iter()
        .flat_map(|l| l.ids.iter().map(|i| (i.clone(), l.name.clone())))
        .collect();

    /* The new laptop (system B). */
    let new_docs = new.join("Documents");
    std::fs::create_dir_all(&new_docs).unwrap();
    for r in pctwin_special::roots::roots_of(b.special(), &new, &|_| None) {
        std::fs::create_dir_all(&r.path).unwrap();
    }
    let mut table = Destinations::new();
    table
        .approve(
            role_label(FolderRole::Documents),
            Approved::MyFolders,
            &new_docs,
        )
        .unwrap();
    let journal = Journal::open_with_record_key(
        &t.path().join("journal.redb"),
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
        own_folder: None,
    };
    let (new_tools, new_hive) = new_settings(b);
    let mut adapter = SettingsAdapter {
        store: Store::new(
            Catalogue::starter().unwrap(),
            Mechanisms {
                system: b.settings(),
                user: new_hive.map(|h| Box::new(h) as Box<dyn Hive>),
                machine: None,
                runner: (b != Kind::Windows)
                    .then(|| Box::new(new_tools.clone()) as Box<dyn Runner>),
                localtime: None,
            },
        ),
        platform: Platform {
            os: b.os(),
            arch: Arch::X86_64,
        },
    };
    // What each catalogue row holds here now, as the executor reads it (by the row's ID).
    let now: BTreeMap<String, Option<Option<pctwin_recipe::SettingValue>>> = Catalogue::starter()
        .unwrap()
        .settings
        .iter()
        .map(|r| {
            let id = r.id.as_str().to_string();
            let v = adapter.store.read(&id).ok();
            (id, v)
        })
        .collect();
    let mut person = NewPerson {
        b,
        rules: rules.clone(),
        extras: None,
        installed: AppList::default(),
        folders: vec![(FolderRole::Documents, new_docs.display().to_string())],
        casks: (b == Kind::Mac).then(casks),
        now,
        plan: None,
        sink: None,
        staging: staging.clone(),
    };
    let mut source = Source::new(
        special.planned.clone(),
        &keys().capture,
        Arc::new(Free),
        work.clone(),
    );

    /* 2. Transfer. */
    let (mut la, mut lb) = mem_pair();
    let (old_live, new_live) = (Live::default(), Live::default());
    let (sent, received) = tokio::join!(
        async {
            use pctwin_move::greeting::{About, Power, Side, greet};
            let mut link = greet(&mut la, &About::here(Power::Unknown, None), Side::Old)
                .await
                .map_err(pctwin_transfer::TransferError::from)?;
            let who = pctwin_move::Who {
                mover: MOVER,
                people: &[],
                extras: &extras_bytes,
                wifi: None,
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
                Some(&mut source),
            )
            .await
        },
        async {
            use pctwin_move::greeting::{About, Power, Side, greet};
            let mut link = greet(&mut lb, &About::here(Power::Unknown, None), Side::New)
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
    drop(source);
    assert!(!work.exists(), "the old laptop's work folder is removed");

    // Every file landed, byte for byte, under a name of its own on this drive.
    let mut landed_names: Vec<String> = Vec::new();
    for (name, bytes) in &files {
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
            panic!("{a:?} to {b:?}: {name:?} did not land: {f:?}");
        };
        let at = new_docs.join(&d.final_path);
        assert_eq!(&std::fs::read(&at).unwrap(), bytes, "{name:?} at {at:?}");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&at).unwrap().permissions().mode();
            assert_eq!(mode & 0o111, 0, "{name:?} never lands able to run");
        }
        // On B's own runner, every landed name is one B holds as it is.
        if b.is_host() {
            let last = Path::new(&d.final_path)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned();
            assert!(
                pctwin_gate::convert_name(&last, b.gate())
                    .changes
                    .is_empty(),
                "{name:?} landed as {last:?}, which {b:?} cannot hold as it is"
            );
        }
        landed_names.push(d.final_path.clone());
    }
    let distinct: std::collections::BTreeSet<String> = landed_names
        .iter()
        .map(|n| {
            if b == Kind::Linux {
                n.clone()
            } else {
                n.to_lowercase()
            }
        })
        .collect();
    assert_eq!(distinct.len(), files.len(), "{landed_names:?}");
    // The old laptop is read only: its files and app data are as they were.
    for (id, src) in &sources {
        let n = record.items.iter().position(|i| i.id == *id).unwrap();
        assert_eq!(&std::fs::read(src).unwrap(), files[n].1);
    }
    assert_eq!(
        std::fs::read(a.code_folder(&old).join("User/keybindings.json")).unwrap(),
        code_before
    );

    /* 3. Rebuild. */
    let plan = person
        .plan
        .take()
        .expect("the plan was shown at the approval");
    let sink = person.sink.take().expect("the app data's choice was made");
    let changes = plan.changes();
    let line = |name: &str| {
        changes
            .iter()
            .find(|c| c.title == Title::Name(name.into()))
            .unwrap_or_else(|| panic!("{name} is on the plan: {changes:#?}"))
    };
    // Apps: the same maker's edition for B, as the counterparts catalogue names it.
    let ff = plan.apps.iter().find(|e| e.app.name == firefox).unwrap();
    match b {
        Kind::Windows => {
            let s = ff.subject();
            assert_eq!(s.system, AppSystem::Windows);
            assert_eq!(s.name, "Mozilla Firefox (x64 en-US)");
            assert!(
                matches!(&s.maker, AppMaker::Windows(sig) if sig.organization.as_deref() == Some("Mozilla Corporation")),
                "{s:?}"
            );
            assert_eq!(ff.winget(), Some("Mozilla.Firefox"));
        }
        Kind::Mac => {
            let s = ff.subject();
            assert_eq!(s.system, AppSystem::Mac);
            assert_eq!(
                s.maker,
                AppMaker::MacTeam {
                    team_id: "43AQ936H96".into()
                }
            );
            assert_eq!(ff.source(), Some("firefox"), "from Firefox's own cask");
        }
        Kind::Linux => {
            // The catalogue names Firefox's Flatpak ID on Linux but not the remote and key it
            // comes from: no maker to check, so the person installs it.
            assert_eq!(line(firefox).needs, Some(Reason::ManualInstall));
        }
    }
    // Spotify: the maker's web version is the choice where no edition is installed.
    let sp = line(spotify);
    assert_eq!(sp.needs, Some(Reason::ManualInstall), "{sp:#?}");
    assert!(
        sp.choices
            .iter()
            .any(|c| c.kind == ChoiceKind::WebVersion
                && c.url.starts_with("https://open.spotify.com")),
        "{sp:#?}"
    );
    // An app no row names: the person's, never another maker's in its place.
    let un = line(unnamed);
    assert_eq!(un.needs, Some(Reason::ManualInstall));
    assert!(un.choices.is_empty(), "{un:#?}");
    assert!(
        plan.apps
            .iter()
            .find(|e| e.app.name == unnamed)
            .is_some_and(|e| e.plan.stand_in.is_none())
    );

    // Settings: an app's own settings never cross between systems; they are said, not moved.
    assert!(
        plan.settings
            .not_moved
            .iter()
            .any(|n| n.why == Why::OtherSystem
                && (n.store.to_lowercase().contains("labwriter")
                    || n.store.contains("PCTwinLabVendor")
                    || n.label.contains("PCTwinLabVendor"))),
        "{:#?}",
        plan.settings.not_moved
    );
    assert!(
        plan.settings
            .moves
            .iter()
            .all(|m| !m.label.to_lowercase().contains("labwriter")
                && !m.label.contains("PCTwinLabVendor")),
        "{:#?}",
        plan.settings.moves
    );

    // App data lands for B.
    let home = Destination::open(&new).unwrap();
    let actor = Actor {
        acting_account: HERE.into(),
        for_account: HERE.into(),
        permission: Permission::OwnFolders,
    };
    let scope = Scope::for_a_move().unwrap();
    let home_old = old.display().to_string();
    let home_new = new.display().to_string();
    let landed = land_all(
        &sink,
        &Landing {
            journal: &journal,
            key: &keys().capture,
            home: &home,
            home_path: &new,
            owner_only: &AppOwnerOnly,
            scope,
            actor: actor.clone(),
            system: b.special(),
            from: a.special(),
            fix: Some(pctwin_fullmove::fixups::FixCtx {
                from: a.settings(),
                to: b.settings(),
                home_old: &home_old,
                home_new: &home_new,
                files: &[],
            }),
        },
        &apps_by_id,
        &Default::default(),
    );
    let code_here = b.code_folder(&new).join("User").join("keybindings.json");
    for l in &landed {
        if l.result.status() == ItemStatus::Copied && l.app.contains("Code") {
            assert!(
                code_here.exists(),
                "{a:?} to {b:?}: VS Code's data is said copied, so it is where VS Code on {b:?} reads it: {landed:#?}"
            );
        }
    }

    adapter.store.remember(plan.settings.book()).unwrap();
    let theme_before = read_row(&mut adapter, b, "appearance.theme");
    let speed_before = read_row(&mut adapter, b, "pointer.speed");

    let files_here = pctwin_fullmove::run::files_landed(
        &plan,
        received.files.iter().filter_map(|f| {
            let item = received.record.items.iter().find(|i| i.id == f.item)?;
            match &f.outcome {
                Some(ReceiveOutcome::Finished(d)) => Some((item, new_docs.join(&d.final_path))),
                _ => None,
            }
        }),
    );
    let data = DataDone {
        picked: Vec::new(),
        landed: landed.iter().map(|l| (l.id.clone(), l.result)).collect(),
        why: landed
            .iter()
            .filter_map(|l| Some((l.id.clone(), l.why.clone()?)))
            .collect(),
        effects: landed
            .iter()
            .filter_map(|l| Some((l.app.clone(), l.effect?)))
            .collect(),
        home: Some(home_new.clone()),
        files: files_here,
        mail: Vec::new(),
    };
    let mut win = WindowsStandIn::default();
    let mut mac = MacStandIn::default();
    let packages = PackageStandIn::default();
    let empty = || AppList::default();
    let key = |_: &str, _: bool| None;
    let mut linux = LinuxInstaller {
        runner: &packages,
        tools: PackageTools {
            pkcon: Some("/usr/bin/pkcon".into()),
            flatpak: Some("/usr/bin/flatpak".into()),
            snap: Some("/usr/bin/snap".into()),
        },
        home: new.clone(),
        session: vec![],
        flatpak_key: &key,
        list: &empty,
    };
    let installer: &mut dyn Installer = match b {
        Kind::Windows => &mut win,
        Kind::Mac => &mut mac,
        Kind::Linux => &mut linux,
    };
    let rebuild_rec = Rebuild::new(&journal, &keys().install, &keys().setting);
    let mut done = rebuild(
        &plan,
        &data,
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
    // Reinstalled only through B's installer, as the same maker's edition.
    match b {
        Kind::Windows => {
            assert_eq!(win.asked, ["Mozilla.Firefox"]);
            assert_eq!(result(firefox).status(), ItemStatus::Reinstalled);
        }
        Kind::Mac => {
            if result(firefox).status() == ItemStatus::Reinstalled {
                assert_eq!(
                    mac.asked,
                    ["firefox"],
                    "reinstalled only through the installer"
                );
            }
            if strict.mac_installs {
                assert_eq!(mac.asked, ["firefox"]);
                assert_eq!(result(firefox).status(), ItemStatus::Reinstalled);
            }
        }
        Kind::Linux => {
            assert!(packages.calls.borrow().is_empty(), "nothing is installed");
            assert_eq!(result(firefox).reason(), Some(Reason::ManualInstall));
        }
    }
    assert_eq!(result(spotify).reason(), Some(Reason::ManualInstall));
    assert_eq!(result(unnamed).reason(), Some(Reason::ManualInstall));
    // Nothing is claimed that did not happen.
    assert!(
        !lab_setting_here(b, &mut adapter, &new_tools),
        "an app's own setting never crosses"
    );
    for (label, _, r) in &done.settings {
        assert_eq!(r.status(), ItemStatus::Converted, "{label}: {r:?}");
    }
    for s in &done.special {
        if s.result.status() == ItemStatus::Copied {
            let Title::Name(n) = &s.title else { panic!() };
            assert!(
                landed
                    .iter()
                    .any(|l| &l.app == n && l.result.status() == ItemStatus::Copied),
                "{n} claimed copied: {landed:#?}"
            );
        }
    }
    if strict.settings {
        let theme_now = read_row(&mut adapter, b, "appearance.theme");
        let speed_now = read_row(&mut adapter, b, "pointer.speed");
        assert_ne!(
            theme_now, theme_before,
            "{a:?} to {b:?}: the dark theme crossed"
        );
        assert_ne!(
            speed_now, speed_before,
            "{a:?} to {b:?}: the pointer speed crossed"
        );
        assert!(
            done.settings
                .iter()
                .any(|(_, _, r)| r.status() == ItemStatus::Converted),
            "{:#?}",
            done.settings
        );
    }
    if strict.data {
        let fixed = std::fs::read_to_string(&code_here).unwrap_or_else(|e| {
            panic!("{a:?} to {b:?}: VS Code's shortcuts are at {code_here:?} ({e}): {landed:#?}")
        });
        assert!(
            fixed.contains(b.shortcut()),
            "{a:?} to {b:?}: the shortcuts are {b:?}'s: {fixed}"
        );
        assert!(
            landed
                .iter()
                .any(|l| l.app.contains("Code") && l.result.status() == ItemStatus::Copied),
            "{landed:#?}"
        );
    }

    /* 4. Undo: the person changes the pointer speed afterwards. */
    person_changes_speed(b, &mut adapter, &new_tools);
    let speed_person = read_row(&mut adapter, b, "pointer.speed");
    let lines = undo::undo(
        &rebuild_rec,
        &mut done.undo,
        &mut adapter,
        Some((&keys().capture, &home)),
        &mut undo::ListOnly,
    )
    .unwrap();
    // An app PCTwin installed is the person's to remove, said as such.
    if b == Kind::Windows {
        assert!(
            done.undo.apps.contains(&firefox.to_string()),
            "{:?}",
            done.undo.apps
        );
        assert!(
            lines
                .iter()
                .any(|l| l.name.contains(firefox) && l.result == undo::Result::RemoveYourself),
            "{lines:#?}"
        );
    }
    // The person's own change is never undone.
    assert_eq!(read_row(&mut adapter, b, "pointer.speed"), speed_person);
    if strict.settings {
        assert_eq!(
            read_row(&mut adapter, b, "appearance.theme"),
            theme_before,
            "put back as it was before the move"
        );
        assert!(
            lines
                .iter()
                .any(|l| matches!(l.result, undo::Result::KeptChanged { .. })),
            "{lines:#?}"
        );
    } else {
        assert_eq!(read_row(&mut adapter, b, "appearance.theme"), theme_before);
    }
    if strict.data {
        if cfg!(windows) || strict.unix_data_undo {
            assert!(
                !code_here.exists(),
                "{a:?} to {b:?}: undo removed the app data that landed"
            );
        } else {
            // Undoing landed app data is built for Windows (pctwin-special's land.rs): on a Mac
            // or Linux it is said not done, and nothing is removed.
            assert!(code_here.exists());
            assert!(
                lines.iter().any(|l| l.name.contains("Code")
                    && matches!(l.result, undo::Result::NotDoneYet { .. })),
                "{lines:#?}"
            );
        }
    }
    // The old laptop's settings are as they were.
    match a {
        Kind::Windows => assert_eq!(
            old_hive
                .get("Software\\PCTwinLabVendor\\Writer", "Theme")
                .unwrap(),
            Some(RegValue::Sz("ocean".into()))
        ),
        Kind::Mac => assert!(
            old_tools
                .0
                .borrow()
                .prefs
                .contains_key("org.fakevendor.labwriter")
        ),
        Kind::Linux => assert!(
            old_tools
                .0
                .borrow()
                .dconf
                .contains_key("/org/fakevendor/labwriter/theme")
        ),
    }
}

/* ---------- The six directions ---------- */

macro_rules! direction {
    ($main:ident, $settings:ident, $data:ident, $a:expr, $b:expr) => {
        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn $main() {
            cross_move($a, $b, Strict::all($b)).await;
        }

        /// The settings catalogue's rows (theme, pointer speed) cross between the two systems:
        /// read by row on the old laptop into the extras (move protocol 9), mapped by
        /// `map::plan`, applied through the executor and journal; undo puts them back and keeps
        /// the person's own change (was an engine gap until 31 October 2026).
        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn $settings() {
            cross_move(
                $a,
                $b,
                Strict {
                    settings: true,
                    ..Strict::default()
                },
            )
            .await;
        }

        /// App data lands placed for the new laptop, decided there from its own catalogue
        /// (`data::placed_here`), its settings files put right for it (`fixups`); undo removes it
        /// on Windows and says "not done" on a Mac or Linux (was an engine gap until 31 October
        /// 2026).
        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn $data() {
            cross_move(
                $a,
                $b,
                Strict {
                    data: true,
                    ..Strict::default()
                },
            )
            .await;
        }
    };
}

direction!(
    windows_to_mac_moves_files_apps_and_undo_honestly,
    windows_to_mac_settings_cross_by_the_catalogue,
    windows_to_mac_app_data_lands_placed_for_the_mac,
    Kind::Windows,
    Kind::Mac
);
direction!(
    mac_to_windows_moves_files_apps_and_undo_honestly,
    mac_to_windows_settings_cross_by_the_catalogue,
    mac_to_windows_app_data_lands_placed_for_windows,
    Kind::Mac,
    Kind::Windows
);
direction!(
    windows_to_linux_moves_files_apps_and_undo_honestly,
    windows_to_linux_settings_cross_by_the_catalogue,
    windows_to_linux_app_data_lands_placed_for_linux,
    Kind::Windows,
    Kind::Linux
);
direction!(
    linux_to_windows_moves_files_apps_and_undo_honestly,
    linux_to_windows_settings_cross_by_the_catalogue,
    linux_to_windows_app_data_lands_placed_for_windows,
    Kind::Linux,
    Kind::Windows
);
direction!(
    mac_to_linux_moves_files_apps_and_undo_honestly,
    mac_to_linux_settings_cross_by_the_catalogue,
    mac_to_linux_app_data_lands_placed_for_linux,
    Kind::Mac,
    Kind::Linux
);
direction!(
    linux_to_mac_moves_files_apps_and_undo_honestly,
    linux_to_mac_settings_cross_by_the_catalogue,
    linux_to_mac_app_data_lands_placed_for_the_mac,
    Kind::Linux,
    Kind::Mac
);

/// On a Mac, the same maker's edition is installed from its cask: whether a route is one PCTwin
/// installs from is decided in one place (`Route::chain`), which the plan and every installer
/// read (was an engine gap until 31 October 2026).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn windows_to_mac_installs_the_same_makers_edition_from_its_cask() {
    cross_move(
        Kind::Windows,
        Kind::Mac,
        Strict {
            mac_installs: true,
            ..Strict::default()
        },
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn linux_to_mac_installs_the_same_makers_edition_from_its_cask() {
    cross_move(
        Kind::Linux,
        Kind::Mac,
        Strict {
            mac_installs: true,
            ..Strict::default()
        },
    )
    .await;
}

/// Engine gap: undoing app data that landed is built for Windows only
/// (`pctwin-special/src/land.rs` `remove_placed`): on a Mac or Linux it is said "not done this
/// time" and nothing is removed. The gate's move-aside-then-check removal for those systems
/// (`pctwin_gate::Destination::check_copy`, `remove_checked`), with its journal record and the
/// Mac's Trash, is not yet joined to the app data landing.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "engine gap: undo of landed app data is built for Windows only"]
async fn undo_removes_landed_app_data_on_mac_and_linux() {
    let here = if cfg!(target_os = "macos") {
        Kind::Mac
    } else {
        Kind::Linux
    };
    let from = if here == Kind::Mac {
        Kind::Linux
    } else {
        Kind::Mac
    };
    cross_move(
        from,
        here,
        Strict {
            data: true,
            unix_data_undo: true,
            ..Strict::default()
        },
    )
    .await;
}
