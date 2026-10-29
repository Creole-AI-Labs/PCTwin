//! App data moves with the move, end to end on one machine through the same engine the app runs:
//! a Firefox-like profile and a Chromium-like profile on the old laptop are found by 2.5, the new
//! laptop chooses what copies (and one secret the person ticked), the old laptop captures each
//! component consistently (SQLite through its online backup), the stream carries them inside the
//! sealed link before the files, the new laptop checks every file into PCTwin's own staging and
//! lands each component through the safety gate and the imports journal. Then:
//!
//! - the browsing history databases open on the new laptop with their rows;
//! - cookies (bound to the old laptop) never move;
//! - saved passwords move only as the person ticked them;
//! - every line ends with one of the six words, never "copied" for something that did not land;
//! - undo removes what landed, keeping a file the person changed after the move.
// Tests make files to test on.
#![allow(clippy::disallowed_methods)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use pctwin_apps::routes::{Target, WingetBy};
use pctwin_fullmove::data::{Landing, Sink, Source, land_all};
use pctwin_fullmove::extras::{Extras, SpecialScan};
use pctwin_fullmove::plan::{FullPlan, NewLaptop};
use pctwin_fullmove::run::{AppResult, DataDone, Installer, rebuild};
use pctwin_gate::{Approved, Destination, Destinations};
use pctwin_journal::{Actor, Journal, Permission, Rebuild, RestorePoint, Scope};
use pctwin_move::landing::LandingRules;
use pctwin_move::{Live, Preview, Steps, Stop};
use pctwin_record::{
    AppArch, AppList, AppSystem, FolderRole, FoundApp, Inclusion, Item, ItemId, ItemKind, ItemName,
    ItemPath, LaptopId, ManagedBy, Owner, PersonTarget, Place, Portability, Record, Storage,
};
use pctwin_settings::System;
use pctwin_special::capture::{Holders, Holding};
use pctwin_transfer::{Channel, ChannelError, LandingPlace, ReceiveOutcome, role_label};
use pctwin_types::ItemStatus;
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

fn put(p: &Path, bytes: &[u8]) {
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, bytes).unwrap();
}

fn sqlite(p: &Path, sql: &str) {
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    let c = rusqlite::Connection::open(p).unwrap();
    c.execute_batch(sql).unwrap();
}

/// A Firefox-like profile: history, bookmarks and preferences, saved passwords under their key
/// database, and cookies.
fn firefox(home: &Path) -> PathBuf {
    let p = home.join("AppData/Roaming/Mozilla/Firefox/Profiles/lab.default-release");
    sqlite(
        &p.join("places.sqlite"),
        "CREATE TABLE moz_places(id INTEGER PRIMARY KEY, url TEXT, title TEXT);
         INSERT INTO moz_places(url, title) VALUES ('https://example.org', 'Example'),
         ('https://example.com', 'Another');",
    );
    sqlite(
        &p.join("cookies.sqlite"),
        "CREATE TABLE moz_cookies(id INTEGER PRIMARY KEY, originAttributes TEXT, name TEXT, value TEXT,
         host TEXT, path TEXT, expiry INTEGER, lastAccessed INTEGER, creationTime INTEGER,
         isSecure INTEGER, isHttpOnly INTEGER);
         INSERT INTO moz_cookies(name, value, host, path, expiry)
         VALUES ('session', 'secret-session-value', 'example.org', '/', 0);",
    );
    sqlite(
        &p.join("key4.db"),
        "CREATE TABLE metaData(id TEXT, item1 BLOB); CREATE TABLE nssPrivate(id INTEGER, a11 BLOB);",
    );
    put(
        &p.join("logins.json"),
        br#"{"logins":[{"hostname":"https://example.org","encryptedUsername":"MDoEEPgAAAAAAAAAAAAAAAAAAAEwFAYIKoZIhvcNAwcECAkJCQkJCQkJ","encryptedPassword":"MDoEEPgAAAAAAAAAAAAAAAAAAAEwFAYIKoZIhvcNAwcECAkJCQkJCQkJ"}]}"#,
    );
    put(&p.join("prefs.js"), b"user_pref(\"browser.lab\", true);\n");
    put(
        &home.join("AppData/Roaming/Mozilla/Firefox/profiles.ini"),
        b"[Profile0]\nName=default-release\nIsRelative=1\nPath=Profiles/lab.default-release\n",
    );
    p
}

/// A Chromium-like profile: history, bookmarks, cookies and saved passwords.
fn chromium(home: &Path) -> PathBuf {
    let root = home.join("AppData/Local/Chromey/User Data");
    put(
        &root.join("Local State"),
        br#"{"os_crypt":{"encrypted_key":"RFBBUEkBAAAAIGludmVudGVkIHdyYXBwZWQga2V5IGJ5dGVz"},"browser":{"x":1}}"#,
    );
    let d = root.join("Default");
    put(
        &d.join("Bookmarks"),
        br#"{"roots":{"bookmark_bar":{"children":[{"name":"Example","url":"https://example.org"}]}},"version":1}"#,
    );
    sqlite(
        &d.join("History"),
        "CREATE TABLE urls(id INTEGER, url TEXT, title TEXT, visit_count INTEGER);
         INSERT INTO urls VALUES (1, 'https://example.org', 'Example', 3);",
    );
    sqlite(
        &d.join("Cookies"),
        "CREATE TABLE cookies(creation_utc INTEGER, host_key TEXT, name TEXT, encrypted_value BLOB, path TEXT, expires_utc INTEGER);
         INSERT INTO cookies VALUES (1, 'example.org', 'sid', x'763130aabbcc', '/', 0);",
    );
    sqlite(
        &d.join("Login Data"),
        "CREATE TABLE logins(origin_url TEXT, username_value TEXT, password_value BLOB);",
    );
    d
}

/// The installer stand-in (no apps in this test).
struct NoApps;
impl Installer for NoApps {
    fn restore_point(&mut self) -> RestorePoint {
        RestorePoint::NotWindows
    }
    fn install(
        &mut self,
        apps: &[&pctwin_fullmove::plan::AppEntry],
        _: &mut dyn FnMut(),
    ) -> Vec<AppResult> {
        apps.iter()
            .map(|_| AppResult::because(pctwin_types::Reason::ManualInstall))
            .collect()
    }
}

/// The new laptop's person: everything to this account; the plan is approved with the copying
/// app data and the secrets the person ticks (`tick`: which secret lines, by app name).
struct NewPerson {
    rules: LandingRules,
    extras: Option<Vec<u8>>,
    plan: Option<FullPlan>,
    picked: Vec<u32>,
    tick: fn(&str) -> bool,
    sink: Option<Sink>,
    staging: PathBuf,
}

fn no_hints(_: &FoundApp) -> Vec<(String, WingetBy)> {
    Vec::new()
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
        let installed = AppList::default();
        let here = NewLaptop {
            system: System::Windows,
            target: Target {
                system: AppSystem::Windows,
                arch: AppArch::UNKNOWN,
                linux: None,
            },
            installed: &installed,
            folders: vec![],
            winget: &no_hints,
        };
        let plan = FullPlan::build(
            record,
            MOVER,
            Some((&x, pctwin_fullmove::extras::digest(&bytes))),
            &here,
            &mut |_, _| Some(None),
        );
        // The person ticks secrets one by one; none is ticked unless they do.
        self.picked = plan
            .special
            .iter()
            .enumerate()
            .filter(|(_, s)| s.secret && (self.tick)(&s.name))
            .map(|(at, _)| at as u32)
            .collect();
        self.sink = Some(Sink::new(plan.choice(&self.picked), self.staging.clone()));
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

fn laptop() -> LaptopId {
    LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap()
}

fn letter() -> Item {
    let owner = Owner::Person {
        account_id: MOVER.into(),
    };
    let place = Place {
        role: FolderRole::Documents,
        storage: Storage::SystemDrive,
    };
    let path = ItemPath::new(vec![ItemName::from_text("letter.txt")]).unwrap();
    Item {
        id: ItemId::derive(&laptop(), &owner, &place, &path),
        kind: ItemKind::File,
        owner,
        place,
        path,
        size_bytes: 8,
        modified_ns: None,
        portability: Portability::Portable,
        managed_by: ManagedBy::Personal,
        download_bytes: None,
        inclusion: Inclusion::Included,
    }
}

fn files_under(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                out.push(
                    p.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
    }
    out.sort();
    out
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_profiles_land_consistent_cookies_never_move_and_secrets_only_when_ticked() {
    let t = tempfile::tempdir().unwrap();
    let old = t.path().join("old");
    let new = t.path().join("new");
    let staging = t.path().join("staging");
    let work = t.path().join("old-work");
    std::fs::create_dir_all(new.join("Documents")).unwrap();
    std::fs::create_dir_all(new.join("AppData/Roaming")).unwrap();
    std::fs::create_dir_all(new.join("AppData/Local")).unwrap();
    std::fs::create_dir_all(&staging).unwrap();
    firefox(&old);
    chromium(&old);
    put(&old.join("Documents/letter.txt"), b"Dear Ada");
    let mover = Owner::Person {
        account_id: MOVER.into(),
    };

    // The old laptop: the 2.5 scan beside the files', then its extras.
    let found = SpecialScan::start_for(pctwin_special::roots::System::Windows, &mover, &old)
        .unwrap()
        .wait(&|| false)
        .expect("the scan finishes");
    let mut extras = Extras::empty(System::Windows);
    extras.with_special(Some(&found));
    let extras_bytes = extras.encode();
    let apps_by_id: BTreeMap<String, String> = found
        .lines
        .iter()
        .flat_map(|l| l.ids.iter().map(|i| (i.clone(), l.name.clone())))
        .collect();
    let mut record = Record::new(laptop());
    record.items = vec![letter()];

    // The new laptop.
    let mut table = Destinations::new();
    table
        .approve(
            role_label(FolderRole::Documents),
            Approved::MyFolders,
            &new.join("Documents"),
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
    };
    let mut person = NewPerson {
        rules,
        extras: None,
        plan: None,
        picked: Vec::new(),
        // The person ticks Firefox's saved passwords (a secret), nothing else.
        tick: |name| name.contains("Firefox") || name.contains("Mozilla"),
        sink: None,
        staging: staging.clone(),
    };
    let mut source = Source::new(
        found.planned.clone(),
        &keys().capture,
        Arc::new(Free),
        work.clone(),
    );

    let (mut a, mut b) = mem_pair();
    let (old_live, new_live) = (Live::default(), Live::default());
    let old_root = old.clone();
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
                |i| Some(old_root.join("Documents").join(i.path.parts()[0].on_disk())),
                &old_live,
                Arc::new(pctwin_transfer::FsOpener),
                &mut back,
                Some(&mut source),
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
    assert!(matches!(
        received.files[0].outcome,
        Some(ReceiveOutcome::Finished(_))
    ));
    drop(source);
    assert!(!work.exists(), "the old laptop's work folder is removed");

    // The old laptop's app data is unchanged (only read).
    let ff = old.join("AppData/Roaming/Mozilla/Firefox/Profiles/lab.default-release");
    let c = rusqlite::Connection::open(ff.join("places.sqlite")).unwrap();
    let n: i64 = c
        .query_row("SELECT COUNT(*) FROM moz_places", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 2);
    drop(c);

    // The new laptop lands what arrived.
    let plan = person.plan.take().unwrap();
    let sink = person.sink.take().unwrap();
    let home = Destination::open(&new).unwrap();
    let actor = Actor {
        acting_account: HERE.into(),
        for_account: HERE.into(),
        permission: Permission::OwnFolders,
    };
    let scope = Scope::for_a_move().unwrap();
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
            system: pctwin_special::roots::System::Windows,
        },
        &apps_by_id,
    );
    assert!(!landed.is_empty());
    let done = DataDone {
        picked: person.picked.clone(),
        landed: landed.iter().map(|l| (l.id.clone(), l.result)).collect(),
        why: landed
            .iter()
            .filter_map(|l| Some((l.id.clone(), l.why.clone()?)))
            .collect(),
        effects: landed
            .iter()
            .filter_map(|l| Some((l.app.clone(), l.effect?)))
            .collect(),
        home: Some(new.display().to_string()),
        files: Vec::new(),
    };
    let mut settings = pctwin_settings::SettingsAdapter {
        store: pctwin_settings::Store::new(
            pctwin_settings::catalogue::Catalogue::starter().unwrap(),
            pctwin_settings::mechanism::Mechanisms {
                system: System::Windows,
                user: Some(Box::new(pctwin_settings::registry::MemHive::default())),
                machine: None,
                runner: None,
                localtime: None,
            },
        ),
        platform: pctwin_recipe::Platform {
            os: pctwin_recipe::Os::Windows,
            arch: pctwin_recipe::Arch::X86_64,
        },
    };
    let rebuild_rec = Rebuild::new(&journal, &keys().install, &keys().setting);
    let mut rebuilt = rebuild(
        &plan,
        &done,
        &mut NoApps,
        &mut settings,
        &rebuild_rec,
        scope,
        &actor,
        laptop(),
        &mut |_| {},
    );

    // Every line ends with one of the six words; "copied" only for what landed, verified.
    let copied: Vec<&String> = landed
        .iter()
        .filter(|l| l.result.status() == ItemStatus::Copied)
        .map(|l| &l.app)
        .collect();
    assert!(!copied.is_empty(), "{landed:#?}");
    for s in &rebuilt.special {
        if s.result.status() == ItemStatus::Copied {
            let pctwin_fullmove::plan::Title::Name(n) = &s.title else {
                panic!()
            };
            assert!(copied.contains(&n), "{n} claimed copied: {landed:#?}");
        }
    }
    let on_new = files_under(&new.join("AppData"));
    // The browsing history databases open on the new laptop with their rows: consistent.
    let places = on_new
        .iter()
        .find(|f| f.ends_with("places.sqlite"))
        .unwrap_or_else(|| panic!("Firefox history landed: {on_new:#?} {landed:#?}"));
    let c = rusqlite::Connection::open(new.join("AppData").join(places)).unwrap();
    let n: i64 = c
        .query_row("SELECT COUNT(*) FROM moz_places", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 2);
    let ok: String = c
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .unwrap();
    assert_eq!(ok, "ok");
    drop(c);
    let history = on_new
        .iter()
        .find(|f| f.ends_with("Default/History"))
        .unwrap_or_else(|| panic!("Chromium history landed: {on_new:#?}"));
    let c = rusqlite::Connection::open(new.join("AppData").join(history)).unwrap();
    let n: i64 = c
        .query_row("SELECT COUNT(*) FROM urls", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 1);
    drop(c);
    // Cookies never move, from either browser.
    assert!(
        !on_new
            .iter()
            .any(|f| f.ends_with("Cookies") || f.ends_with("cookies.sqlite")),
        "{on_new:#?}"
    );
    // Chromium's saved passwords were not ticked: they stayed on the old laptop.
    assert!(
        !on_new.iter().any(|f| f.ends_with("Login Data")),
        "{on_new:#?}"
    );
    let chromey_secret = plan
        .special
        .iter()
        .enumerate()
        .find(|(_, s)| s.secret && s.name.contains("Chromey"));
    if let Some((at, _)) = chromey_secret {
        assert!(!person.picked.contains(&(at as u32)));
    }
    assert!(
        rebuilt
            .special
            .iter()
            .all(|s| s.result.status() != ItemStatus::Failed || s.result.reason().is_some())
    );
    // R1: saved sign-ins to websites (cookies) are tied to the old laptop, so for each browser
    // the outcome says to sign in to websites again, on a line that needs the person: never only
    // "copied", as if the sign-ins came across.
    for browser in ["Chromey", "Mozilla"] {
        let sites: Vec<_> = rebuilt
            .special
            .iter()
            .filter(|s| {
                matches!(&s.title, pctwin_fullmove::plan::Title::Name(n) if n.contains(browser))
                    && s.sites
            })
            .collect();
        assert!(
            !sites.is_empty(),
            "{browser}: no line says to sign in to websites again: {:#?}",
            rebuilt.special
        );
        assert!(
            sites
                .iter()
                .all(|s| s.result.status() == ItemStatus::NeedsAction),
            "{sites:#?}"
        );
    }
    // A line copied whole carries no reason to act; one that needs the person and landed is said
    // with why (never the generic "follow the steps" with no steps).
    for s in &rebuilt.special {
        if s.result.status() == ItemStatus::Copied {
            assert_eq!(s.why, None, "{s:?}");
        }
    }

    // The person edits a landed file afterwards; undo removes the rest and keeps theirs.
    let prefs = on_new.iter().find(|f| f.ends_with("prefs.js")).cloned();
    if let Some(prefs) = &prefs {
        std::fs::write(
            new.join("AppData").join(prefs),
            b"user_pref(\"edited\", 1);\n",
        )
        .unwrap();
    }
    let mut adapter = settings;
    let lines = pctwin_fullmove::undo::undo(
        &rebuild_rec,
        &mut rebuilt.undo,
        &mut adapter,
        Some((&keys().capture, &home)),
        &mut pctwin_fullmove::undo::ListOnly,
    )
    .unwrap();
    assert!(!lines.is_empty());
    let after = files_under(&new.join("AppData"));
    // Undoing brought-in app data is built for Windows first (pctwin-special's land.rs);
    // elsewhere it says so and removes nothing.
    assert!(
        cfg!(not(windows)) || !after.iter().any(|f| f.ends_with("places.sqlite")),
        "undo removed what landed: {after:#?}"
    );
    if let Some(prefs) = prefs {
        assert!(
            after.contains(&prefs),
            "the person's edit is kept: {after:#?}"
        );
    }
}

/// Says Chromey's files are held open until the person closes it.
struct OpenUntilClosed(Arc<std::sync::atomic::AtomicBool>);
impl Holders for OpenUntilClosed {
    fn holding(&self, files: &[PathBuf]) -> Holding {
        let chromey = files
            .iter()
            .any(|f| f.to_string_lossy().contains("Chromey"));
        if chromey && !self.0.load(std::sync::atomic::Ordering::SeqCst) {
            Holding::Held {
                by: vec!["chromey.exe".into()],
            }
        } else {
            Holding::Free
        }
    }
}

/// Runs the choice and the stream between `source` and `sink` in memory.
async fn stream(source: &mut Source, sink: &mut Sink) {
    use pctwin_move::extras::{DataIn, receive_choice, receive_data, send_choice, send_data_every};
    let (mut a, mut b) = mem_pair();
    send_choice(&mut b, &sink.choice()).await.unwrap();
    let choice = receive_choice(&mut a).await.unwrap();
    pctwin_move::extras::DataOut::chosen(source, choice);
    let (sent, got) = tokio::join!(
        send_data_every(&mut a, source, std::time::Duration::from_millis(50)),
        receive_data(&mut b, sink)
    );
    sent.unwrap();
    got.unwrap();
}

/// An app open on the old laptop: the person is told which, closes it and tries again, and its
/// data then comes; told "go on without", it ends needs-action, never copied.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_app_open_on_the_old_laptop_is_named_and_tried_again_once_closed() {
    // Gap 2: also with the app's folder already on the new laptop (`taken`): it lands beside it,
    // and the result says so with the folder's new name, never a bare "needs action".
    for (close_it, taken) in [(true, false), (false, false), (true, true)] {
        let t = tempfile::tempdir().unwrap();
        let old = t.path().join("old");
        let new = t.path().join("new");
        std::fs::create_dir_all(new.join("AppData/Local")).unwrap();
        if taken {
            std::fs::create_dir_all(new.join("AppData/Local/Chromey")).unwrap();
            std::fs::write(new.join("AppData/Local/Chromey/mine.txt"), b"already here").unwrap();
        }
        chromium(&old);
        let mover = Owner::Person {
            account_id: MOVER.into(),
        };
        let found = SpecialScan::start(&mover, &old)
            .unwrap()
            .wait(&|| false)
            .unwrap();
        let plan = FullPlan {
            special: found.lines.clone(),
            ..FullPlan::default()
        };
        let choice = plan.choice(&[]);
        assert!(!choice.copy.is_empty());
        let closed = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let asked: Arc<std::sync::Mutex<Vec<Vec<String>>>> = Default::default();
        let ask = {
            let (closed, asked) = (closed.clone(), asked.clone());
            Box::new(move |names: &[String]| {
                asked.lock().unwrap().push(names.to_vec());
                if close_it {
                    closed.store(true, std::sync::atomic::Ordering::SeqCst);
                }
                close_it
            }) as pctwin_fullmove::data::AskClose
        };
        let mut source = Source::new(
            found.planned.clone(),
            &keys().capture,
            Arc::new(OpenUntilClosed(closed.clone())),
            t.path().join("work"),
        )
        .asking(ask);
        let mut sink = Sink::new(choice, t.path().join("staging"));
        stream(&mut source, &mut sink).await;
        let asked = asked.lock().unwrap().clone();
        assert_eq!(asked.len(), 1, "asked once, naming the app: {asked:?}");
        assert!(asked[0].iter().any(|n| n.contains("Chromey")), "{asked:?}");
        let journal = Journal::open_with_record_key(
            &t.path().join("j.redb"),
            pctwin_journal::keystore::TagKey::from_bytes([0x5a; 32]),
        )
        .unwrap();
        let home = Destination::open(&new).unwrap();
        let names: BTreeMap<String, String> = found
            .lines
            .iter()
            .flat_map(|l| l.ids.iter().map(|i| (i.clone(), l.name.clone())))
            .collect();
        let landed = land_all(
            &sink,
            &Landing {
                journal: &journal,
                key: &keys().capture,
                home: &home,
                home_path: &new,
                owner_only: &AppOwnerOnly,
                scope: Scope::for_a_move().unwrap(),
                actor: Actor {
                    acting_account: HERE.into(),
                    for_account: HERE.into(),
                    permission: Permission::OwnFolders,
                },
                system: pctwin_special::roots::System::Windows,
            },
            &names,
        );
        let copied = landed
            .iter()
            .filter(|l| l.result.status() == ItemStatus::Copied)
            .count();
        if taken {
            assert_eq!(
                copied, 0,
                "never over the folder already there: {landed:#?}"
            );
            assert!(
                landed.iter().any(|l| matches!(
                    &l.why,
                    Some(pctwin_fullmove::data::DataWhy::Beside { folder })
                        if folder.contains("(from old laptop)")
                )),
                "{landed:#?}"
            );
            assert_eq!(
                std::fs::read(new.join("AppData/Local/Chromey/mine.txt")).unwrap(),
                b"already here"
            );
        } else if close_it {
            assert!(copied > 0, "{landed:#?}");
            assert!(
                new.join("AppData/Local/Chromey/User Data/Default/History")
                    .is_file()
            );
        } else {
            assert_eq!(copied, 0, "{landed:#?}");
            assert!(
                landed
                    .iter()
                    .all(|l| l.result.status() == ItemStatus::NeedsAction
                        && l.words.contains("open")
                        && l.why == Some(pctwin_fullmove::data::DataWhy::AppOpen)),
                "{landed:#?}"
            );
        }
    }
}

/// Sixth review, finding 2: a move that ends before its app data lands (a stop, a lost link, the
/// app closed) leaves nothing staged on the new laptop, a ticked secret least of all; and a staging
/// folder a crash left behind is swept away at the next start.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn staged_app_data_never_outlives_the_move_that_brought_it() {
    let t = tempfile::tempdir().unwrap();
    let old = t.path().join("old");
    firefox(&old);
    let mover = Owner::Person {
        account_id: MOVER.into(),
    };
    let found = SpecialScan::start(&mover, &old)
        .unwrap()
        .wait(&|| false)
        .unwrap();
    let plan = FullPlan {
        special: found.lines.clone(),
        ..FullPlan::default()
    };
    // The person ticks every secret, so the secret's files are staged too.
    let picked: Vec<u32> = plan
        .special
        .iter()
        .enumerate()
        .filter(|(_, s)| s.secret)
        .map(|(at, _)| at as u32)
        .collect();
    assert!(!picked.is_empty(), "a secret to tick");
    let data = t.path().join("PCTwin data");
    let staging = data.join(format!("{}abc", pctwin_fullmove::data::STAGING_PREFIX));
    std::fs::create_dir_all(&staging).unwrap();
    let mut source = Source::new(
        found.planned.clone(),
        &keys().capture,
        Arc::new(Free),
        t.path().join("work"),
    );
    let mut sink = Sink::new(plan.choice(&picked), staging.clone());
    stream(&mut source, &mut sink).await;
    assert!(
        !files_under(&staging).is_empty(),
        "the app data, the secret's included, was staged"
    );
    // The move ends here, without landing.
    drop(sink);
    assert!(!staging.exists(), "nothing staged outlives the move");

    // A staging folder left by a crash is swept at the next start; nothing else is touched.
    let left = data.join(format!("{}def", pctwin_fullmove::data::STAGING_PREFIX));
    put(&left.join("x/logins.json"), b"{}");
    put(&data.join("journal"), b"kept");
    assert_eq!(pctwin_fullmove::data::sweep_staging(&data), 1);
    assert!(!left.exists());
    assert!(data.join("journal").exists());
}
