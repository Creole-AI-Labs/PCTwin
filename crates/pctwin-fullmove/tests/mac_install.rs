//! The Mac install chain (src/mac.rs) end to end through stand-ins for the fetch process and the
//! Mac's own tools (`hdiutil`, `ditto`, `codesign`, `spctl`, `test`), which build and read real
//! folders here, so the chain's every decision is proven on any system. GitHub's Mac runners run
//! the same chain against the real tools.
// Tests make files to test on.
#![allow(clippy::disallowed_methods)]

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use pctwin_apps::after::Todo;
use pctwin_apps::fetch::{Outcome, Request};
use pctwin_apps::routes::{AppPlan, Card, Disposition, Route};
use pctwin_fullmove::mac::MacInstaller;
use pctwin_fullmove::plan::AppEntry;
use pctwin_fullmove::run::Installer;
use pctwin_maker::run::{Call, Ran, RunError, Runner};
use pctwin_record::{AppArch, AppMaker, AppSystem, FoundApp, FoundWhere, Owner};
use pctwin_types::{ItemStatus, Reason};

const TEAM: &str = "SKMME9E2Y8";
const ID: &str = "org.example.painter";

/// What a stand-in bundle is: written inside it so the stand-in tools read it back.
#[derive(Clone)]
struct Bundle {
    id: &'static str,
    version: &'static str,
    team: &'static str,
    arch: &'static str,
    min_system: Option<&'static str>,
    broken: bool,
}

fn painter() -> Bundle {
    Bundle {
        id: ID,
        version: "4.3.0",
        team: TEAM,
        arch: "universal (x86_64 arm64)",
        min_system: None,
        broken: false,
    }
}

fn write_bundle(at: &Path, b: &Bundle) {
    let contents = at.join("Contents");
    std::fs::create_dir_all(&contents).unwrap();
    let min = b
        .min_system
        .map(|m| format!("<key>LSMinimumSystemVersion</key><string>{m}</string>"))
        .unwrap_or_default();
    std::fs::write(
        contents.join("Info.plist"),
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?><plist version=\"1.0\"><dict>\
             <key>CFBundleIdentifier</key><string>{}</string>\
             <key>CFBundleName</key><string>Painter</string>\
             <key>CFBundleShortVersionString</key><string>{}</string>{min}</dict></plist>",
            b.id, b.version
        ),
    )
    .unwrap();
    std::fs::write(contents.join("_team"), b.team).unwrap();
    std::fs::write(contents.join("_arch"), b.arch).unwrap();
    if b.broken {
        std::fs::write(contents.join("_broken"), "").unwrap();
    }
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        let t = to.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            copy_tree(&e.path(), &t);
        } else {
            std::fs::copy(e.path(), t).unwrap();
        }
    }
}

/// The Mac's tools, standing in: each call is kept; the bundle inside every download is `inside`.
struct Mac {
    inside: Bundle,
    app_name: &'static str,
    admin: bool,
    licence: bool,
    gatekeeper_offline: bool,
    /// Gatekeeper accepts it as a Developer ID app Apple has not notarised.
    not_notarised: bool,
    /// The copy PCTwin makes is changed after copying (its signature then fails).
    tamper_copy: bool,
    /// Another app appears at the place while PCTwin copies.
    appears: bool,
    calls: RefCell<Vec<Call>>,
    mounted: RefCell<Vec<PathBuf>>,
}

impl Mac {
    fn new() -> Self {
        Mac {
            inside: painter(),
            app_name: "Painter.app",
            admin: true,
            licence: false,
            gatekeeper_offline: false,
            not_notarised: false,
            tamper_copy: false,
            appears: false,
            calls: RefCell::new(Vec::new()),
            mounted: RefCell::new(Vec::new()),
        }
    }

    fn ran(code: i32, stdout: String, stderr: String) -> Result<Ran, RunError> {
        Ok(Ran {
            code: Some(code),
            stdout,
            stderr,
        })
    }

    fn calls_to(&self, program: &str) -> Vec<Vec<String>> {
        self.calls
            .borrow()
            .iter()
            .filter(|c| c.program == Path::new(program))
            .map(|c| {
                c.args
                    .iter()
                    .map(|a| a.to_string_lossy().into_owned())
                    .collect()
            })
            .collect()
    }
}

impl Runner for Mac {
    fn run(&self, call: &Call) -> Result<Ran, RunError> {
        self.calls.borrow_mut().push(call.clone());
        // Every tool gets a fixed environment and starts at the root.
        assert!(
            call.env
                .iter()
                .any(|(k, v)| k == "PATH" && v == "/usr/bin:/bin:/usr/sbin:/sbin")
                || call.env.is_empty()
        );
        let a: Vec<String> = call
            .args
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        let program = call.program.to_string_lossy().into_owned();
        let last = PathBuf::from(a.last().cloned().unwrap_or_default());
        match program.as_str() {
            "/bin/test" => Self::ran(if self.admin { 0 } else { 1 }, String::new(), String::new()),
            "/usr/bin/hdiutil" if a[0] == "attach" => {
                if self.licence {
                    return Self::ran(
                        1,
                        String::new(),
                        "hdiutil: attach canceled: licence agreement declined".into(),
                    );
                }
                let place =
                    PathBuf::from(&a[a.iter().position(|x| x == "-mountrandom").unwrap() + 1]);
                let mount = place.join("dmg.Ab12");
                write_bundle(&mount.join(self.app_name), &self.inside);
                self.mounted.borrow_mut().push(mount.clone());
                let m = mount.to_string_lossy().replace('\\', "/");
                Self::ran(
                    0,
                    format!(
                        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><plist version=\"1.0\"><dict><key>system-entities</key><array>\
                         <dict><key>dev-entry</key><string>/dev/disk4</string></dict>\
                         <dict><key>dev-entry</key><string>/dev/disk4s1</string><key>mount-point</key><string>{m}</string></dict>\
                         </array></dict></plist>"
                    ),
                    String::new(),
                )
            }
            "/usr/bin/hdiutil" => {
                self.mounted.borrow_mut().retain(|m| *m != Path::new(&a[1]));
                Self::ran(0, String::new(), String::new())
            }
            "/usr/bin/ditto" if a[0] == "-x" => {
                write_bundle(&PathBuf::from(&a[3]).join(self.app_name), &self.inside);
                Self::ran(0, String::new(), String::new())
            }
            "/usr/bin/ditto" => {
                let to = Path::new(&a[1]);
                copy_tree(Path::new(&a[0]), to);
                if self.tamper_copy {
                    std::fs::write(to.join("Contents").join("_broken"), "").unwrap();
                }
                if self.appears {
                    let place = to
                        .parent()
                        .unwrap()
                        .parent()
                        .unwrap()
                        .join(to.file_name().unwrap());
                    std::fs::create_dir_all(&place).unwrap();
                    std::fs::write(place.join("theirs"), "another app").unwrap();
                }
                Self::ran(0, String::new(), String::new())
            }
            "/usr/bin/codesign" if a[0] == "-dv" => {
                let c = last.join("Contents");
                let team = std::fs::read_to_string(c.join("_team")).unwrap_or_default();
                let arch = std::fs::read_to_string(c.join("_arch")).unwrap_or_default();
                Self::ran(
                    0,
                    String::new(),
                    format!(
                        "Format=app bundle with Mach-O {arch}\nAuthority=Developer ID Application: Example ({team})\nTeamIdentifier={team}\n"
                    ),
                )
            }
            "/usr/bin/codesign" => {
                if last.join("Contents").join("_broken").exists() {
                    Self::ran(
                        1,
                        String::new(),
                        format!(
                            "{}: a sealed resource is missing or invalid",
                            last.display()
                        ),
                    )
                } else {
                    Self::ran(0, String::new(), String::new())
                }
            }
            "/usr/sbin/spctl" if self.gatekeeper_offline => Self::ran(
                1,
                String::new(),
                "spctl: could not reach the network".into(),
            ),
            "/usr/sbin/spctl" => Self::ran(
                0,
                String::new(),
                format!(
                    "{}: accepted\nsource={}\norigin=Developer ID Application: Example ({TEAM})",
                    last.display(),
                    if self.not_notarised {
                        "Developer ID"
                    } else {
                        "Notarized Developer ID"
                    }
                ),
            ),
            other => panic!("no stand-in for {other} {a:?}"),
        }
    }
}

fn old_app() -> FoundApp {
    FoundApp {
        name: "Painter".into(),
        version: Some("4.2.0".into()),
        publisher: None,
        owner: Owner::Person {
            account_id: "501".into(),
        },
        system: AppSystem::Mac,
        found: FoundWhere::MacBundle {
            bundle_id: ID.into(),
            folder: "Painter.app".into(),
            app_store_id: None,
        },
        arch: AppArch {
            known: true,
            x64: true,
            arm64: true,
            x86: false,
        },
        maker: AppMaker::MacTeam {
            team_id: TEAM.into(),
        },
        web: None,
        update_feed: None,
        installers: vec![],
        notes: vec![],
    }
}

fn cask(token: &str, url: &str, sha: Option<[u8; 32]>) -> Route {
    Route::Cask {
        token: token.into(),
        version: "4.3.0".into(),
        url: url.into(),
        sha256: sha,
        auto_updates: true,
        app: "Painter.app".into(),
    }
}

fn entry(routes: Vec<Route>) -> AppEntry {
    let app = old_app();
    AppEntry {
        plan: AppPlan {
            disposition: Disposition::Install,
            stand_in: None,
            routes,
            flags: vec![],
            card: Card {
                name: app.name.clone(),
                version: app.version.clone(),
                maker: None,
                web: None,
                licence_key: false,
                sign_in: false,
                registration: vec![],
                why: None,
                choices: vec![],
                trail: None,
            },
        },
        app,
    }
}

/// What the fetch process is asked, and writes: a disk image unless the address ends `.zip`.
struct Downloads {
    asked: RefCell<Vec<Request>>,
    answer: Option<Outcome>,
}

impl Downloads {
    fn new() -> Self {
        Downloads {
            asked: RefCell::new(Vec::new()),
            answer: None,
        }
    }

    fn fetch(&self, req: &Request) -> Outcome {
        self.asked.borrow_mut().push(req.clone());
        if let Some(a) = &self.answer {
            return a.clone();
        }
        let bytes = if req.url.ends_with(".zip") {
            b"PK\x03\x04zip".to_vec()
        } else {
            let mut d = vec![0u8; 1024];
            d[512..516].copy_from_slice(b"koly");
            d
        };
        std::fs::write(&req.file, bytes).unwrap();
        Outcome::Done {
            size: 1024,
            sha256: String::new(),
            from: req.url.clone(),
            resumed: false,
        }
    }
}

struct Laptop {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Laptop {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::fs::create_dir_all(root.join("Applications")).unwrap();
        std::fs::create_dir_all(root.join("home")).unwrap();
        Laptop { _dir: dir, root }
    }

    fn installer<'a>(
        &self,
        mac: &'a Mac,
        fetch: &'a dyn Fn(&Request) -> Outcome,
    ) -> MacInstaller<'a> {
        MacInstaller {
            runner: mac,
            fetch,
            staging: self.root.join("fetch").join("files"),
            work: self.root.join("work"),
            home: self.root.join("home"),
            system_apps: self.root.join("Applications"),
            arm64: true,
            rosetta: true,
            macos: "15.1".into(),
            allow_metered: true,
        }
    }
}

fn install(i: &mut MacInstaller<'_>, e: &AppEntry) -> pctwin_fullmove::run::AppResult {
    let mut n = 0;
    let out = i.install(&[e], &mut || n += 1);
    assert_eq!(n, 1, "one step of progress for each app");
    out.into_iter().next().unwrap()
}

#[test]
fn a_cask_disk_image_is_placed_in_applications_and_checked_where_it_landed() {
    let lap = Laptop::new();
    let mac = Mac::new();
    let dl = Downloads::new();
    let fetch = |r: &Request| dl.fetch(r);
    let mut i = lap.installer(&mac, &fetch);
    let sha = [0xab; 32];
    let r = install(
        &mut i,
        &entry(vec![cask(
            "painter",
            "https://example.com/p.dmg",
            Some(sha),
        )]),
    );
    assert_eq!(r.result.status(), ItemStatus::Reinstalled, "{r:?}");
    assert!(r.installed);
    assert!(r.todo.is_empty());
    let placed = lap.root.join("Applications").join("Painter.app");
    assert!(placed.join("Contents").join("Info.plist").is_file());
    // The download was asked for those very bytes, into the fetch process's own folder.
    let asked = dl.asked.borrow();
    assert_eq!(asked.len(), 1);
    assert_eq!(
        asked[0].expected,
        Some(pctwin_apps::fetch::Expected::Sha256("ab".repeat(32)))
    );
    assert_eq!(
        asked[0].file.parent(),
        Some(lap.root.join("fetch").join("files").as_path())
    );
    // Attached read-only, hidden from the Finder, never agreeing to a licence, and put away.
    let attach = &mac.calls_to("/usr/bin/hdiutil")[0];
    for flag in ["-readonly", "-nobrowse", "-noautoopen", "-plist"] {
        assert!(attach.contains(&flag.to_string()), "{attach:?}");
    }
    let stdin = mac
        .calls
        .borrow()
        .iter()
        .find(|c| c.program == Path::new("/usr/bin/hdiutil"))
        .unwrap()
        .stdin
        .clone();
    assert_eq!(stdin.as_deref(), Some(&b"qn\n"[..]));
    assert!(
        mac.mounted.borrow().is_empty(),
        "the disk image is detached"
    );
    // The download and PCTwin's own folder are gone.
    assert!(!asked[0].file.exists());
    assert!(!lap.root.join("work").exists());
    // Checked inside the image before it was placed, and again where it landed.
    let verified = mac
        .calls_to("/usr/bin/codesign")
        .into_iter()
        .filter(|a| a[0] == "--verify")
        .count();
    assert_eq!(verified, 2);
}

#[test]
fn a_zip_cask_for_a_person_who_is_not_an_administrator_lands_in_their_own_applications() {
    let lap = Laptop::new();
    let mut mac = Mac::new();
    mac.admin = false;
    let dl = Downloads::new();
    let fetch = |r: &Request| dl.fetch(r);
    let mut i = lap.installer(&mac, &fetch);
    let r = install(
        &mut i,
        &entry(vec![cask("painter", "https://example.com/p.zip", None)]),
    );
    assert_eq!(r.result.status(), ItemStatus::Reinstalled, "{r:?}");
    assert!(
        lap.root
            .join("home")
            .join("Applications")
            .join("Painter.app")
            .is_dir()
    );
    assert!(!lap.root.join("Applications").join("Painter.app").exists());
    assert_eq!(
        dl.asked.borrow()[0].expected,
        None,
        "a cask without a checksum: the Team ID decides"
    );
    assert!(mac.calls_to("/usr/bin/hdiutil").is_empty());
}

#[test]
fn another_team_another_bundle_or_a_broken_signature_is_never_placed() {
    for (what, bundle) in [
        (
            "another team",
            Bundle {
                team: "ZZZZZZZZZZ",
                ..painter()
            },
        ),
        (
            "another app",
            Bundle {
                id: "org.example.other",
                ..painter()
            },
        ),
        (
            "a broken signature",
            Bundle {
                broken: true,
                ..painter()
            },
        ),
    ] {
        let lap = Laptop::new();
        let mut mac = Mac::new();
        mac.inside = bundle;
        let dl = Downloads::new();
        let fetch = |r: &Request| dl.fetch(r);
        let mut i = lap.installer(&mac, &fetch);
        let r = install(
            &mut i,
            &entry(vec![cask("painter", "https://example.com/p.dmg", None)]),
        );
        assert_eq!(
            r.result.reason(),
            Some(Reason::NotVerified),
            "{what}: {r:?}"
        );
        assert!(!r.installed, "{what}");
        assert!(
            !lap.root.join("Applications").join("Painter.app").exists(),
            "{what}"
        );
        assert!(
            mac.calls_to("/usr/bin/ditto").is_empty(),
            "{what}: nothing copied"
        );
        assert!(mac.mounted.borrow().is_empty(), "{what}: detached");
    }
}

#[test]
fn gatekeeper_unreachable_means_not_proven_so_not_placed() {
    let lap = Laptop::new();
    let mut mac = Mac::new();
    mac.gatekeeper_offline = true;
    let dl = Downloads::new();
    let fetch = |r: &Request| dl.fetch(r);
    let mut i = lap.installer(&mac, &fetch);
    let r = install(
        &mut i,
        &entry(vec![cask("painter", "https://example.com/p.dmg", None)]),
    );
    assert_eq!(r.result.reason(), Some(Reason::NotVerified));
    assert!(!lap.root.join("Applications").join("Painter.app").exists());
}

#[test]
fn the_next_cask_is_tried_when_one_is_not_the_app() {
    let lap = Laptop::new();
    let mac = Mac::new();
    let dl = Downloads::new();
    let fetch_bad = |r: &Request| {
        let n = dl.asked.borrow().len();
        if n == 0 {
            dl.asked.borrow_mut().push(r.clone());
            return Outcome::NotExpectedBytes;
        }
        dl.fetch(r)
    };
    let mut i = lap.installer(&mac, &fetch_bad);
    let r = install(
        &mut i,
        &entry(vec![
            cask("painter-mirror", "https://example.com/a.dmg", Some([1; 32])),
            cask("painter", "https://example.com/b.dmg", None),
        ]),
    );
    assert_eq!(r.result.status(), ItemStatus::Reinstalled, "{r:?}");
    assert_eq!(dl.asked.borrow().len(), 2);
}

#[test]
fn an_app_already_at_its_place_is_never_replaced() {
    let lap = Laptop::new();
    let mac = Mac::new();
    let mine = lap.root.join("Applications").join("Painter.app");
    std::fs::create_dir_all(&mine).unwrap();
    std::fs::write(mine.join("mine"), "the person's own").unwrap();
    let dl = Downloads::new();
    let fetch = |r: &Request| dl.fetch(r);
    let mut i = lap.installer(&mac, &fetch);
    let r = install(
        &mut i,
        &entry(vec![cask("painter", "https://example.com/p.dmg", None)]),
    );
    assert_eq!(r.result.reason(), Some(Reason::FinishByHand));
    assert!(dl.asked.borrow().is_empty(), "nothing downloaded");
    assert_eq!(
        std::fs::read_to_string(mine.join("mine")).unwrap(),
        "the person's own"
    );
}

#[test]
fn waiting_for_free_wifi_downloads_nothing_and_the_person_installs_it_later() {
    let lap = Laptop::new();
    let mac = Mac::new();
    let mut dl = Downloads::new();
    dl.answer = Some(Outcome::MeteredWait);
    let fetch = |r: &Request| dl.fetch(r);
    let mut i = lap.installer(&mac, &fetch);
    i.allow_metered = false;
    let r = install(
        &mut i,
        &entry(vec![
            cask("painter", "https://example.com/p.dmg", None),
            cask("painter-2", "https://example.com/q.dmg", None),
        ]),
    );
    assert_eq!(r.result.reason(), Some(Reason::ManualInstall));
    let asked = dl.asked.borrow();
    assert_eq!(
        asked.len(),
        1,
        "no second cask on a connection that may charge"
    );
    assert!(!asked[0].allow_metered);
}

#[test]
fn a_disk_image_that_asks_to_agree_to_a_licence_is_left_for_the_person() {
    let lap = Laptop::new();
    let mut mac = Mac::new();
    mac.licence = true;
    let dl = Downloads::new();
    let fetch = |r: &Request| dl.fetch(r);
    let mut i = lap.installer(&mac, &fetch);
    let r = install(
        &mut i,
        &entry(vec![cask("painter", "https://example.com/p.dmg", None)]),
    );
    assert_eq!(r.result.reason(), Some(Reason::ManualInstall));
    assert!(!lap.root.join("Applications").join("Painter.app").exists());
}

#[test]
fn an_intel_only_app_on_apple_silicon_says_rosetta_and_needs_it_installed() {
    for rosetta in [true, false] {
        let lap = Laptop::new();
        let mut mac = Mac::new();
        mac.inside = Bundle {
            arch: "thin (x86_64)",
            ..painter()
        };
        let dl = Downloads::new();
        let fetch = |r: &Request| dl.fetch(r);
        let mut i = lap.installer(&mac, &fetch);
        i.rosetta = rosetta;
        let r = install(
            &mut i,
            &entry(vec![cask("painter", "https://example.com/p.dmg", None)]),
        );
        assert!(r.todo.contains(&Todo::Rosetta), "{r:?}");
        let placed = lap.root.join("Applications").join("Painter.app").exists();
        if rosetta {
            assert_eq!(r.result.status(), ItemStatus::Reinstalled);
            assert!(r.installed && placed);
        } else {
            // Checked before anything is placed: never placed, the person's step.
            assert_eq!(r.result.reason(), Some(Reason::FinishByHand));
            assert!(!r.installed && !placed);
        }
    }
    // On an Intel Mac the same app is native: nothing to say.
    let lap = Laptop::new();
    let mut mac = Mac::new();
    mac.inside = Bundle {
        arch: "thin (x86_64)",
        ..painter()
    };
    let dl = Downloads::new();
    let fetch = |r: &Request| dl.fetch(r);
    let mut i = lap.installer(&mac, &fetch);
    i.arm64 = false;
    i.rosetta = false;
    let r = install(
        &mut i,
        &entry(vec![cask("painter", "https://example.com/p.dmg", None)]),
    );
    assert_eq!(r.result.status(), ItemStatus::Reinstalled);
    assert!(r.todo.is_empty());
}

#[test]
fn an_app_needing_a_newer_macos_or_older_than_the_persons_is_not_reinstalled() {
    let lap = Laptop::new();
    let mut mac = Mac::new();
    mac.inside = Bundle {
        min_system: Some("26.0"),
        ..painter()
    };
    let dl = Downloads::new();
    let fetch = |r: &Request| dl.fetch(r);
    let mut i = lap.installer(&mac, &fetch);
    let r = install(
        &mut i,
        &entry(vec![cask("painter", "https://example.com/p.dmg", None)]),
    );
    assert_eq!(r.result.reason(), Some(Reason::NoEquivalent), "{r:?}");
    assert!(!r.installed);
    assert!(
        !lap.root.join("Applications").join("Painter.app").exists(),
        "never placed"
    );

    let lap = Laptop::new();
    let mut mac = Mac::new();
    mac.inside = Bundle {
        version: "4.1.0",
        min_system: Some("12.0"),
        ..painter()
    };
    let dl = Downloads::new();
    let fetch = |r: &Request| dl.fetch(r);
    let mut i = lap.installer(&mac, &fetch);
    let r = install(
        &mut i,
        &entry(vec![cask("painter", "https://example.com/p.dmg", None)]),
    );
    assert_eq!(
        r.result.reason(),
        Some(Reason::FinishByHand),
        "older than the person's"
    );
    assert!(!r.installed);
    assert!(
        !lap.root.join("Applications").join("Painter.app").exists(),
        "never placed"
    );
    assert!(mac.calls_to("/usr/bin/ditto").is_empty(), "nothing copied");

    // An app for Apple silicon only, going to an Intel Mac, would not run: never placed.
    let lap = Laptop::new();
    let mut mac = Mac::new();
    mac.inside = Bundle {
        arch: "thin (arm64)",
        ..painter()
    };
    let dl = Downloads::new();
    let fetch = |r: &Request| dl.fetch(r);
    let mut i = lap.installer(&mac, &fetch);
    i.arm64 = false;
    let r = install(
        &mut i,
        &entry(vec![cask("painter", "https://example.com/p.dmg", None)]),
    );
    assert_eq!(r.result.reason(), Some(Reason::NoEquivalent), "{r:?}");
    assert!(!lap.root.join("Applications").join("Painter.app").exists());
}

#[test]
fn an_app_for_neither_processor_of_an_apple_silicon_mac_is_never_placed() {
    // An old 32-bit Intel app: Rosetta cannot run it either.
    let lap = Laptop::new();
    let mut mac = Mac::new();
    mac.inside = Bundle {
        arch: "thin (i386)",
        ..painter()
    };
    let dl = Downloads::new();
    let fetch = |r: &Request| dl.fetch(r);
    let mut i = lap.installer(&mac, &fetch);
    let r = install(
        &mut i,
        &entry(vec![cask("painter", "https://example.com/p.dmg", None)]),
    );
    assert_eq!(r.result.reason(), Some(Reason::NoEquivalent), "{r:?}");
    assert!(!r.todo.contains(&Todo::Rosetta));
    assert!(!lap.root.join("Applications").join("Painter.app").exists());
}

#[test]
fn a_developer_id_app_apple_has_not_notarised_is_never_placed() {
    let lap = Laptop::new();
    let mut mac = Mac::new();
    mac.not_notarised = true;
    let dl = Downloads::new();
    let fetch = |r: &Request| dl.fetch(r);
    let mut i = lap.installer(&mac, &fetch);
    let r = install(
        &mut i,
        &entry(vec![cask("painter", "https://example.com/p.dmg", None)]),
    );
    assert_eq!(r.result.reason(), Some(Reason::NotVerified), "{r:?}");
    assert!(!lap.root.join("Applications").join("Painter.app").exists());
}

#[test]
fn an_old_app_with_no_team_never_matches_a_download_with_none() {
    let lap = Laptop::new();
    let mut mac = Mac::new();
    // The download's signature names no team; neither did the old laptop's.
    mac.inside = Bundle {
        team: "not set",
        ..painter()
    };
    let dl = Downloads::new();
    let fetch = |r: &Request| dl.fetch(r);
    let mut i = lap.installer(&mac, &fetch);
    let mut e = entry(vec![cask("painter", "https://example.com/p.dmg", None)]);
    e.app.maker = AppMaker::NotRead {
        why: "Its signature names no developer team.".into(),
    };
    let r = install(&mut i, &e);
    assert_eq!(r.result.reason(), Some(Reason::NotVerified), "{r:?}");
    assert!(!lap.root.join("Applications").join("Painter.app").exists());
}

#[test]
fn the_copy_is_checked_where_it_was_made_and_a_changed_copy_is_never_placed() {
    let lap = Laptop::new();
    let mut mac = Mac::new();
    mac.tamper_copy = true;
    let dl = Downloads::new();
    let fetch = |r: &Request| dl.fetch(r);
    let mut i = lap.installer(&mac, &fetch);
    let r = install(
        &mut i,
        &entry(vec![cask("painter", "https://example.com/p.dmg", None)]),
    );
    assert_eq!(r.result.reason(), Some(Reason::NotVerified), "{r:?}");
    let apps = lap.root.join("Applications");
    assert!(!apps.join("Painter.app").exists());
    // PCTwin's own copy folder is gone; nothing else was there to touch.
    assert_eq!(std::fs::read_dir(&apps).unwrap().count(), 0);
}

#[test]
fn an_app_that_appears_at_the_place_while_copying_is_never_replaced_or_removed() {
    let lap = Laptop::new();
    let mut mac = Mac::new();
    mac.appears = true;
    let dl = Downloads::new();
    let fetch = |r: &Request| dl.fetch(r);
    let mut i = lap.installer(&mac, &fetch);
    let r = install(
        &mut i,
        &entry(vec![cask("painter", "https://example.com/p.dmg", None)]),
    );
    assert_eq!(r.result.reason(), Some(Reason::FinishByHand), "{r:?}");
    assert!(!r.installed);
    let apps = lap.root.join("Applications");
    let theirs = apps.join("Painter.app");
    assert_eq!(
        std::fs::read_to_string(theirs.join("theirs")).unwrap(),
        "another app"
    );
    assert!(
        !theirs.join("Contents").exists(),
        "nothing of PCTwin's went into it"
    );
    // Only PCTwin's own copy folder was removed.
    let left: Vec<_> = std::fs::read_dir(&apps)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(left, [std::ffi::OsString::from("Painter.app")]);
}

#[test]
fn a_link_inside_the_download_is_never_followed() {
    let d = tempfile::tempdir().unwrap();
    let root = d.path();
    std::fs::create_dir_all(root.join("Tools").join("Painter.app")).unwrap();
    std::fs::write(root.join("File.app"), b"").unwrap();
    let inside = |p: &str| pctwin_fullmove::mac::app_inside(p).unwrap();
    assert!(pctwin_fullmove::mac::reach_inside(root, &inside("Tools/Painter.app")).is_some());
    assert!(
        pctwin_fullmove::mac::reach_inside(root, &inside("File.app")).is_none(),
        "a file"
    );
    assert!(pctwin_fullmove::mac::reach_inside(root, &inside("Gone.app")).is_none());
    let elsewhere = d.path().join("elsewhere");
    std::fs::create_dir_all(elsewhere.join("Painter.app")).unwrap();
    // A folder in the download that is a link (to /Applications, say). Making a link needs a
    // right Windows does not always give; the Mac and Linux runners always make it.
    #[cfg(unix)]
    let linked = std::os::unix::fs::symlink(&elsewhere, root.join("Linked")).is_ok();
    #[cfg(windows)]
    let linked = std::os::windows::fs::symlink_dir(&elsewhere, root.join("Linked")).is_ok();
    if linked {
        assert!(pctwin_fullmove::mac::reach_inside(root, &inside("Linked/Painter.app")).is_none());
        #[cfg(unix)]
        std::os::unix::fs::symlink(elsewhere.join("Painter.app"), root.join("Direct.app")).unwrap();
        #[cfg(unix)]
        assert!(pctwin_fullmove::mac::reach_inside(root, &inside("Direct.app")).is_none());
    }
}

#[test]
fn an_app_with_no_cask_is_the_persons_to_install() {
    let lap = Laptop::new();
    let mac = Mac::new();
    let dl = Downloads::new();
    let fetch = |r: &Request| dl.fetch(r);
    let mut i = lap.installer(&mac, &fetch);
    let r = install(&mut i, &entry(vec![Route::MacAppStore { adam_id: 42 }]));
    assert_eq!(r.result.reason(), Some(Reason::ManualInstall));
    assert!(dl.asked.borrow().is_empty());
    assert!(
        mac.calls
            .borrow()
            .iter()
            .all(|c| c.program == Path::new("/bin/test"))
    );
}
