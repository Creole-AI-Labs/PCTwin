//! The full move on the screens (Phase 2 joining work, 27 October 2026): apps, settings and
//! special categories joined to the move of files and people (`pctwin-fullmove`).
//!
//! - The old laptop reads its settings and special categories (only reading) and sends them with
//!   its list ([`extras_here`]).
//! - The new laptop plans every change before the approval and shows it on the one approval
//!   screen (M23), each with what undo can do for it ([`plan_here`], [`RebuildPlanView`]); the
//!   plan's digest joins the fingerprint the person approves ([`approved_fingerprint`]).
//! - After the files, the new laptop applies the plan: settings through the executor and the
//!   effect journal (each read back), apps through the installer, every item ending with one of
//!   the six result words ([`rebuild_here`], [`RebuiltView`]).
//! - Undo runs through the flow guard: the undo preview (R05) and its result (R06), each press
//!   checked again ([`fullmove_undo`]).
//!
//! The role keys are made once, when the app starts ([`new_state`]); only this module holds them.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use pctwin_fullmove::extras::{Extras, NotMovedCount, OldFolder, SpecialStep};
use pctwin_fullmove::plan::{Action, ChangeNote, FullPlan, Kind, NewLaptop, Promise, Title};
use pctwin_fullmove::run::{Installer, NoInstaller, Progress, UndoBook};
use pctwin_journal::{Actor, Journal, Permission, Rebuild, RestorePoint, Scope};
use pctwin_record::{AppArch, AppSystem, LaptopId, Owner, Record};
use pctwin_scan::FolderLookup;
use pctwin_settings::catalogue::Catalogue;
use pctwin_settings::mechanism::Mechanisms;
use pctwin_settings::{SettingsAdapter, Store};
use pctwin_types::{ItemStatus, Reason};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

use crate::guard::{AppGuard, DisplayState, IntentKind, IntentName, Measured, Unit, Update};
use crate::guard_views::{
    ChangeAction, ScreenView, UndoLine, UndoPlan, UndoPlanLine, UndoPromise, UndoResult, bounded,
    name,
};

/// Where rebuilding apps and settings is, after the files.
pub const REBUILD_EVENT: &str = "move-rebuild";
/// The undo the screens opened is over (the person went back).
pub const UNDO_CLOSED_EVENT: &str = "move-undo-closed";

/// The full move's state: the role keys, and each move's undo book made since PCTwin started
/// (by its move's ID; also kept on disk, one file per move: [`crate::undo_books`]).
pub struct FullMoveState {
    keys: pctwin_grant::RoleKeys,
    books: Mutex<std::collections::BTreeMap<String, UndoBook>>,
    epoch: AtomicU64,
}

pub type AppFullMove = Arc<FullMoveState>;

/// The state, with the role keys made now (once per program; `None` if they were made already,
/// and the program must then not start).
/// One state per program: building the app again in the same program (its permission tests do)
/// is given the same keys, never a second set.
static STATE: std::sync::OnceLock<Option<AppFullMove>> = std::sync::OnceLock::new();

pub fn new_state() -> Option<AppFullMove> {
    STATE.get_or_init(made).clone()
}

/// The 2.5 role's key, for the program's whole life (the capture runs on a thread of its own).
fn capture_key() -> Option<&'static pctwin_grant::CaptureKey> {
    STATE.get()?.as_ref().map(|s| &s.keys.capture)
}

/// The new laptop: the old laptop's progress with the app data (`move-appdata`).
pub const APPDATA_EVENT: &str = "move-appdata";
/// The old laptop: apps to close before their data can be read (`move-appdata-open`).
pub const APPDATA_OPEN_EVENT: &str = "move-appdata-open";

/// The apps the person is asked to close on the old laptop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AppsOpen {
    pub apps: Vec<String>,
}

fn made() -> Option<AppFullMove> {
    // Windows' own list of dangerous file types adds caution to the settings judge; set before
    // anything reads the system's facts.
    #[cfg(windows)]
    pctwin_settings::judge::use_system_danger_check(pctwin_helper_os::file_type_is_dangerous);
    // What this laptop says can run is read once (a few seconds), as PCTwin starts, never per
    // setting.
    let _ = std::thread::Builder::new()
        .name("pctwin-facts".into())
        .spawn(|| {
            let _ = pctwin_settings::judge::Facts::system();
        });
    Some(Arc::new(FullMoveState {
        keys: pctwin_grant::RoleKeys::mint()?,
        books: Mutex::new(std::collections::BTreeMap::new()),
        // Sessions of the undo screens only grow, from when the app started.
        epoch: AtomicU64::new(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(1),
        ),
    }))
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/* ---------- The old laptop ---------- */

/// The old laptop: the extras for the person `mover`, read only: their settings, their folders
/// here, their special categories as `special` found them (`None`: not in time), and how many
/// links and junctions the scan found among their files (`links`: never followed, never moved).
/// `settings`: whether the person chose to bring their apps, settings and app data (when not, none
/// of the settings found are sent).
pub fn extras_here(
    special: Option<&pctwin_fullmove::extras::SpecialFound>,
    links: u64,
    settings: bool,
) -> Vec<u8> {
    let folders = pctwin_scan::find_special_folders()
        .into_iter()
        .filter_map(|f| match f {
            FolderLookup::Found(f) => Some(OldFolder {
                role: f.role,
                path: f.path.display().to_string(),
            }),
            FolderLookup::Missing { .. } => None,
        })
        .collect();
    let mut x = pctwin_fullmove::extras::here(folders);
    x.with_special(special);
    x.links = links;
    if !settings {
        x.settings.clear();
        x.left_out_by_person = true;
    }
    x.encode()
}

/* ---------- The plan (M23) ---------- */

/// One change on the approval screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeView {
    pub title: Title,
    pub kind: Kind,
    pub action: ChangeAction,
    pub undo: UndoPromise,
    pub count: u64,
    pub needs: Option<Reason>,
    pub special: Option<SpecialStep>,
    pub files: u64,
    /// A secret the person may tick to bring: its line (none unless ticked).
    pub secret: Option<u32>,
    pub source: Option<String>,
    /// What else the person should know (an older version here, saved sign-ins to websites).
    pub note: Option<ChangeNote>,
}

/// What the approval screen shows of the apps, settings and special categories.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RebuildPlanView {
    pub changes: Vec<ChangeView>,
    pub more: u64,
    pub not_moved: Vec<NotMovedCount>,
    pub special_not_moved: u64,
    /// Links and junctions among the person's files: never followed, never moved.
    pub links: u64,
    /// The system's administrator prompt is asked once for the move.
    pub prompt: bool,
    /// A Windows restore point is asked for before the first change.
    pub restore_point: bool,
}

fn action_of(a: Action) -> ChangeAction {
    match a {
        Action::Install => ChangeAction::Install,
        Action::Merge => ChangeAction::Merge,
        Action::Replace => ChangeAction::Replace,
        Action::LeaveAlone => ChangeAction::LeaveAlone,
    }
}

fn promise_of(p: Promise) -> UndoPromise {
    match p {
        Promise::CanReverse => UndoPromise::CanReverse,
        Promise::MustKeep => UndoPromise::MustKeep,
        Promise::RemoveYourself => UndoPromise::RemoveYourself,
    }
}

/// winget, where Windows puts it for the signed-in person (`None`: not on this laptop, so PCTwin
/// does not install apps here and asks for no administrator).
fn winget_here() -> Option<std::path::PathBuf> {
    if !cfg!(windows) {
        return None;
    }
    let local = std::env::var_os("LOCALAPPDATA")?;
    let path = pctwin_apps::routes::winget::winget_path(std::path::Path::new(&local));
    path.exists().then_some(path)
}

/// Whether this laptop installs apps itself (Windows with winget: the administrator helper's chain).
fn installs_here() -> bool {
    winget_here().is_some()
}

/// This laptop's processor, as winget names it.
#[cfg(windows)]
fn winget_arch() -> &'static str {
    if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "x64"
    }
}

/// The administrator helper for the install chain: started once, with the system's own prompt,
/// for the sealed plan.
#[cfg(windows)]
struct AppHelper {
    me: String,
    laptop: LaptopId,
    client: Option<pctwin_helper::Client<pctwin_helper::client::Channel>>,
}

#[cfg(windows)]
impl pctwin_fullmove::install::Helper for AppHelper {
    fn start(
        &mut self,
        plan: &pctwin_helper::broker::actions::ApprovedPlan,
    ) -> Result<(), pctwin_fullmove::install::HelperDown> {
        use pctwin_fullmove::install::HelperDown;
        let seal = format!("{}:{}", plan.generation, plan.digest());
        let mut client = match crate::others::launch_elevated(
            &self.me,
            &[],
            self.laptop,
            Some(&seal),
            &|| false,
        ) {
            Ok(Some(c)) => c,
            Ok(None) => return Err(HelperDown::Declined),
            Err(_) => return Err(HelperDown::Failed),
        };
        match client.seal_plan(plan) {
            Ok(Ok(())) | Ok(Err(pctwin_helper::broker::Denial::Replayed)) => {
                self.client = Some(client);
                Ok(())
            }
            _ => Err(HelperDown::Failed),
        }
    }
    fn stage(
        &mut self,
        artifact: u64,
        sha256: pctwin_helper::broker::actions::Hex32,
        size: u64,
        mut from: &mut dyn std::io::Read,
    ) -> Result<(), String> {
        let client = self.client.as_mut().ok_or("the helper is not running")?;
        match client.stage(artifact, sha256, size, &mut from) {
            Ok(Ok(())) => Ok(()),
            Ok(Err(why)) => Err(why.to_string()),
            Err(e) => Err(e.to_string()),
        }
    }
    fn act(
        &mut self,
        request: &pctwin_helper::broker::BrokerRequest,
    ) -> Result<pctwin_helper::Acted, String> {
        let client = self.client.as_mut().ok_or("the helper is not running")?;
        client.act(request).map_err(|e| e.to_string())
    }
    fn bye(&mut self) {
        if let Some(c) = self.client.take() {
            let _ = c.bye();
        }
    }
}

/// Windows' own list of apps installed for everyone.
#[cfg(windows)]
struct WindowsList;

#[cfg(windows)]
impl pctwin_fullmove::install::Machine for WindowsList {
    fn entries(&self) -> Vec<pctwin_apps::found::windows::RawEntry> {
        pctwin_apps::found::windows::machine_entries()
    }
}

/// The view of `plan`.
pub fn plan_view(plan: &FullPlan) -> RebuildPlanView {
    let changes: Vec<ChangeView> = plan
        .changes()
        .into_iter()
        .map(|c| ChangeView {
            title: match c.title {
                Title::Name(n) => Title::Name(name(&n)),
                k => k,
            },
            kind: c.kind,
            action: action_of(c.action),
            undo: promise_of(c.undo),
            count: c.count,
            needs: c.needs,
            special: c.special,
            files: c.files,
            secret: c.secret,
            source: c.source.map(|n| name(&n)),
            note: c.note.map(|n| match n {
                ChangeNote::OlderHere { version } => ChangeNote::OlderHere {
                    version: name(&version),
                },
                n => n,
            }),
        })
        .collect();
    // Each kind is bounded on its own, so many settings never hide an app or a step to take.
    let mut more = 0;
    let mut kept = Vec::new();
    for kind in [Kind::App, Kind::Settings, Kind::Special] {
        let (lines, cut) = bounded(changes.iter().filter(|c| c.kind == kind).cloned().collect());
        kept.extend(lines);
        more += cut;
    }
    let changes = kept;
    let asks = installs_here() && plan.changes_anything();
    RebuildPlanView {
        changes,
        more,
        not_moved: plan.not_moved.clone(),
        special_not_moved: plan.special_not_moved,
        links: plan.links,
        prompt: asks,
        restore_point: asks && cfg!(windows),
    }
}

/// The fingerprint the person approves: the record's and the full plan's together, so an
/// approval of one plan cannot approve another.
pub fn approved_fingerprint(record_fingerprint: &str, plan: &FullPlan) -> String {
    let mut h = blake3::Hasher::new_derive_key("PCTwin approved full move 2026-10-27 v1");
    h.update(record_fingerprint.as_bytes());
    h.update(&plan.digest());
    h.finalize().to_hex().to_string()
}

fn this_target() -> pctwin_apps::routes::Target {
    let system = if cfg!(windows) {
        AppSystem::Windows
    } else if cfg!(target_os = "macos") {
        AppSystem::Mac
    } else {
        AppSystem::Linux
    };
    pctwin_apps::routes::Target {
        system,
        arch: AppArch {
            known: true,
            x86: false,
            x64: cfg!(target_arch = "x86_64"),
            arm64: cfg!(target_arch = "aarch64"),
        },
        linux: None,
    }
}

/// This laptop's settings, through its real mechanisms, knowing `plan`'s places.
fn settings_here(plan: &FullPlan) -> Option<SettingsAdapter> {
    let system = pctwin_fullmove::extras::this_system();
    let mut store = Store::new(Catalogue::starter().ok()?, Mechanisms::real(system));
    store.remember(plan.settings.book()).ok()?;
    Some(SettingsAdapter {
        store,
        platform: pctwin_recipe::Platform {
            os: pctwin_fullmove::run::os_of(system),
            arch: if cfg!(target_arch = "aarch64") {
                pctwin_recipe::Arch::Aarch64
            } else {
                pctwin_recipe::Arch::X86_64
            },
        },
    })
}

/// The new laptop: the full plan for the old laptop's `record` and `extras` (as they arrived),
/// for the person `mover` there, moving to the signed-in person `me` here.
pub fn plan_here(record: &Record, mover: &str, extras: Option<&[u8]>, me: &str) -> FullPlan {
    let people: Vec<(String, Option<std::path::PathBuf>)> = pctwin_scan::list_people()
        .into_iter()
        .map(|p| (p.account_id, Some(p.home)))
        .collect();
    let installed = pctwin_apps::found::read_here(&people, me);
    let folders = pctwin_scan::find_special_folders()
        .into_iter()
        .filter_map(|f| match f {
            FolderLookup::Found(f) => Some((f.role, f.path.display().to_string())),
            FolderLookup::Missing { .. } => None,
        })
        .collect();
    let decoded = extras.and_then(|b| Some((Extras::decode(b).ok()?, b)));
    // Extras that arrived but could not be read are said on the plan, never dropped silently.
    let refused = extras.is_some_and(|b| !b.is_empty()) && decoded.is_none();
    // Where each app comes from: winget searched as the person, a few at a time, for the mover's
    // apps this laptop does not have yet.
    let hints = match winget_here() {
        Some(winget) => {
            let owner = Owner::Person {
                account_id: mover.to_string(),
            };
            let wanted: Vec<&pctwin_record::FoundApp> = record
                .apps
                .found
                .iter()
                .filter(|a| a.owner == owner || a.owner == Owner::Shared)
                .filter(|a| {
                    !matches!(
                        pctwin_fullmove::plan::here_of(a, &installed),
                        pctwin_apps::routes::Here::Present { .. }
                    )
                })
                .collect();
            pctwin_fullmove::install::winget_hints(
                &wanted,
                &pctwin_maker::run::SystemRunner,
                &winget,
            )
        }
        None => Default::default(),
    };
    let no_hints = |a: &pctwin_record::FoundApp| {
        hints
            .get(&pctwin_fullmove::install::hint_key(a))
            .cloned()
            .unwrap_or_default()
    };
    let here = NewLaptop {
        system: pctwin_fullmove::extras::this_system(),
        target: this_target(),
        installed: &installed,
        folders,
        winget: &no_hints,
    };
    let mut reader: Option<Option<SettingsAdapter>> = None;
    let mut current = |plan: &pctwin_settings::generic::FoundPlan, key: &str| {
        let adapter = reader.get_or_insert_with(|| {
            let full = FullPlan {
                settings: plan.clone(),
                ..FullPlan::default()
            };
            settings_here(&full)
        });
        let a = adapter.as_mut()?;
        a.store.read(key).ok()
    };
    let mut plan = FullPlan::build(
        record,
        mover,
        decoded
            .as_ref()
            .map(|(x, b)| (x, pctwin_fullmove::extras::digest(b))),
        &here,
        &mut current,
    );
    plan.extras_refused = refused;
    plan
}

/* ---------- After the files ---------- */

/// One line of what the full move did, beside the files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RebuiltLine {
    pub title: Title,
    pub kind: Kind,
    pub count: u64,
    pub status: ItemStatus,
    pub reason: Option<Reason>,
    /// What else the person should know or do for it, each a word the screens translate.
    pub notes: Vec<LineNote>,
}

/// One thing to know or do for an outcome line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "note", rename_all = "kebab-case")]
pub enum LineNote {
    /// Sign in to the app.
    SignIn,
    /// Enter the app's licence key, or activate it.
    LicenceKey,
    /// What the old laptop's own list held about its licence, to activate it again.
    #[serde(rename_all = "camelCase")]
    Registration {
        owner: Option<String>,
        company: Option<String>,
        product_id: Option<String>,
    },
    /// Other programs appeared with it; the person can remove them.
    Extras { apps: Vec<String> },
    /// Saved sign-ins to websites or accounts in it stayed tied to the old laptop.
    Sites,
    /// Why its app data needs the person.
    Data(pctwin_fullmove::data::DataWhy),
}

/// An app already on this laptop, left as it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HereLine {
    pub name: String,
    /// The version here, when it is older than the old laptop's.
    pub older: Option<String>,
}

/// The notes for an app's to-do list (a restart is said by the line's own reason).
fn todo_notes(todo: &[pctwin_apps::after::Todo]) -> Vec<LineNote> {
    use pctwin_apps::after::Todo;
    let text = |t: &Option<String>| t.as_deref().map(name);
    todo.iter()
        .filter_map(|t| match t {
            Todo::SignIn => Some(LineNote::SignIn),
            Todo::LicenceKey => Some(LineNote::LicenceKey),
            Todo::Registration {
                owner,
                company,
                product_id,
            } if owner.is_some() || company.is_some() || product_id.is_some() => {
                Some(LineNote::Registration {
                    owner: text(owner),
                    company: text(company),
                    product_id: text(product_id),
                })
            }
            Todo::Registration { .. } | Todo::Restart => None,
            Todo::Extras(apps) => Some(LineNote::Extras {
                apps: bounded(apps.iter().map(|a| name(a)).collect()).0,
            }),
        })
        .collect()
}

/// The notes for an app data line.
fn data_notes(s: &pctwin_fullmove::run::SpecialDone) -> Vec<LineNote> {
    use pctwin_fullmove::data::DataWhy;
    let mut out = Vec::new();
    if s.sites {
        out.push(LineNote::Sites);
    }
    if let Some(why) = &s.why {
        out.push(LineNote::Data(match why {
            DataWhy::Beside { folder } => DataWhy::Beside {
                folder: name(folder),
            },
            DataWhy::Placed { folder } => DataWhy::Placed {
                folder: name(folder),
            },
            w => w.clone(),
        }));
    }
    out
}

/// The restore point's honest answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RestoreWord {
    Made,
    EarlierExists,
    NotMade,
    NotWindows,
    /// Nothing needed one (no app installed, no setting changed), or this version does not ask.
    NotAsked,
}

/// What the outcome (R01) shows of the apps, settings and special categories.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RebuiltView {
    pub lines: Vec<RebuiltLine>,
    pub more: u64,
    /// The apps already on this laptop, left as they are (not items of the move: outside the
    /// result words, each said).
    pub here: Vec<HereLine>,
    pub here_more: u64,
    /// PCTwin installed at least one app: fresh installs start signed out.
    pub fresh: bool,
    pub restore_point: RestoreWord,
    /// The settings stopped part way (the journal could not be written).
    pub stopped: bool,
    /// Undo can put something back.
    pub can_undo: bool,
    /// The move's own ID, for undoing this move and no other (`None`: nothing to undo).
    pub undo_id: Option<String>,
}

/// The new laptop, after the files: rebuilds `plan` (the approved one) for the signed-in person
/// `me`, journaling in `journal`, and remembers what undo needs.
pub fn rebuild_here<R: Runtime>(
    app: &AppHandle<R>,
    plan: &FullPlan,
    journal: &Journal,
    me: &str,
    data: Option<(&pctwin_fullmove::data::Sink, Vec<u32>)>,
    files: Vec<(String, String)>,
) -> RebuiltView {
    let state = app.state::<AppFullMove>();
    let (Ok(laptop), Ok(scope)) = (LaptopId::generate(), Scope::for_a_move()) else {
        // Nothing is changed without its own record: every group says so.
        return RebuiltView {
            lines: Vec::new(),
            more: 0,
            here: Vec::new(),
            here_more: 0,
            fresh: false,
            restore_point: RestoreWord::NotAsked,
            stopped: true,
            can_undo: false,
            undo_id: None,
        };
    };
    let actor = Actor {
        acting_account: me.to_string(),
        for_account: me.to_string(),
        permission: Permission::OwnFolders,
    };
    let ledger = Rebuild::new(journal, &state.keys.install, &state.keys.setting);
    // The app data that arrived: landed now, in the person's home, through the gate and the
    // imports journal; whatever did not arrive whole ends needs-action.
    let data_done = match data {
        Some((sink, picked)) => {
            land_data(sink, picked, journal, &state.keys.capture, scope, &actor)
        }
        None => pctwin_fullmove::run::DataDone::default(),
    };
    // What landed where: a moved setting points only at a file that is there.
    let data_done = pctwin_fullmove::run::DataDone { files, ..data_done };
    // Windows with winget: the real chain (winget as the person, then the administrator helper
    // once). Elsewhere: every app needs the person, and no administrator is asked for.
    let mut no_installer = NoInstaller {
        why: "this laptop has no way for PCTwin to install apps yet",
    };
    #[cfg(windows)]
    let mut helper = AppHelper {
        me: me.to_string(),
        laptop,
        client: None,
    };
    #[cfg(windows)]
    let machine = WindowsList;
    #[cfg(windows)]
    let mut nonce = || {
        let mut b = [0u8; 16];
        getrandom::fill(&mut b).ok().map(|()| b)
    };
    #[cfg(windows)]
    let work = {
        let mut b = [0u8; 8];
        let _ = getrandom::fill(&mut b);
        let tag: String = b.iter().map(|x| format!("{x:02x}")).collect();
        std::env::temp_dir().join(format!("pctwin-installs-{tag}"))
    };
    #[cfg(windows)]
    let mut chain = winget_here().map(|winget| {
        pctwin_fullmove::install::WingetInstaller::new(
            winget,
            winget_arch(),
            work,
            &pctwin_maker::run::SystemRunner,
            &mut helper,
            &machine,
            &mut nonce,
        )
    });
    #[cfg(windows)]
    let installer: &mut dyn Installer = match chain.as_mut() {
        Some(c) => c,
        None => &mut no_installer,
    };
    #[cfg(not(windows))]
    let installer: &mut dyn Installer = &mut no_installer;
    let mut progress = |step: Progress| {
        let _ = app.emit(REBUILD_EVENT, step);
    };
    let done = match settings_here(plan) {
        Some(mut settings) => pctwin_fullmove::run::rebuild(
            plan,
            &data_done,
            &mut *installer,
            &mut settings,
            &ledger,
            scope,
            &actor,
            laptop,
            &mut progress,
        ),
        None => {
            // No settings mechanism here: nothing is changed; settings say so.
            let empty = FullPlan {
                apps: plan.apps.clone(),
                special: plan.special.clone(),
                special_searched: plan.special_searched,
                special_not_checked: plan.special_not_checked,
                from: plan.from,
                ..FullPlan::default()
            };
            let mut none = NoSettings;
            let mut done = pctwin_fullmove::run::rebuild(
                &empty,
                &data_done,
                &mut *installer,
                &mut none,
                &ledger,
                scope,
                &actor,
                laptop,
                &mut progress,
            );
            done.settings = plan
                .groups
                .iter()
                .map(|g| {
                    (
                        g.label.clone(),
                        g.count,
                        pctwin_types::ItemResult::because(Reason::NotVerified),
                    )
                })
                .collect();
            done
        }
    };
    let mut lines = Vec::new();
    for (n, a) in &done.apps {
        lines.push(RebuiltLine {
            title: Title::Name(name(n)),
            kind: Kind::App,
            count: 0,
            status: a.result.status(),
            reason: a.result.reason(),
            notes: todo_notes(&a.todo),
        });
    }
    for (label, count, r) in &done.settings {
        lines.push(RebuiltLine {
            title: Title::Name(name(label)),
            kind: Kind::Settings,
            count: *count,
            status: r.status(),
            reason: r.reason(),
            notes: Vec::new(),
        });
    }
    for s in &done.special {
        lines.push(RebuiltLine {
            title: match &s.title {
                Title::Name(n) => Title::Name(name(n)),
                k => k.clone(),
            },
            kind: Kind::Special,
            count: 0,
            status: s.result.status(),
            reason: s.result.reason(),
            notes: data_notes(s),
        });
    }
    let (here, here_more) = bounded(
        done.here
            .iter()
            .map(|(n, older)| HereLine {
                name: name(n),
                older: older.as_deref().map(name),
            })
            .collect(),
    );
    let fresh = done.apps.iter().any(|(_, a)| a.installed);
    // Needs the person first, then failed, then done: what to act on comes first.
    lines.sort_by_key(|l| match l.status {
        ItemStatus::NeedsAction => 0,
        ItemStatus::Failed => 1,
        _ => 2,
    });
    let (lines, more) = bounded(lines);
    let restore_point = match &done.restore_point {
        None => RestoreWord::NotAsked,
        Some(RestorePoint::Made { .. }) => RestoreWord::Made,
        Some(RestorePoint::EarlierExists { .. }) => RestoreWord::EarlierExists,
        Some(RestorePoint::NotWindows) => RestoreWord::NotWindows,
        Some(RestorePoint::NotMade { .. }) if !installs_here() => RestoreWord::NotAsked,
        Some(RestorePoint::NotMade { .. }) => RestoreWord::NotMade,
    };
    let can_undo =
        !done.undo.recipes.is_empty() || !done.undo.apps.is_empty() || !done.undo.landed.is_empty();
    // One book per move, never over another move's: kept here and beside the journal, so Undo
    // still works after PCTwin restarts, for this move and every earlier one.
    let undo_id = can_undo.then(|| done.undo.id());
    if can_undo {
        lock(&state.books).insert(done.undo.id(), done.undo.clone());
        if let Some(folder) = books_folder(app) {
            let _ = crate::undo_books::save(&folder, &done.undo);
        }
    }
    RebuiltView {
        lines,
        more,
        here,
        here_more,
        fresh,
        restore_point,
        stopped: done.stopped.is_some(),
        can_undo,
        undo_id,
    }
}

/// Where secrets land owner-only: the system's own private folder.
struct AppOwnerOnly;

impl pctwin_special::land::OwnerOnly for AppOwnerOnly {
    fn private_folder(&self, path: &std::path::Path) -> std::io::Result<()> {
        #[cfg(windows)]
        return pctwin_helper_os::private_folder(path)
            .map_err(|e| std::io::Error::other(format!("{e:?}")));
        #[cfg(unix)]
        return pctwin_special::land::UnixOwnerOnly.private_folder(path);
    }
}

/// The signed-in person's home folder here.
fn home_here() -> Option<std::path::PathBuf> {
    pctwin_scan::list_people()
        .into_iter()
        .find(|p| p.is_me)
        .map(|p| p.home)
}

/// Lands the app data in `sink` (the secrets the person ticked: `picked`) and says how each
/// component ended; the staging folder is removed afterwards.
fn land_data(
    sink: &pctwin_fullmove::data::Sink,
    picked: Vec<u32>,
    journal: &Journal,
    key: &pctwin_grant::CaptureKey,
    scope: Scope,
    actor: &Actor,
) -> pctwin_fullmove::run::DataDone {
    let mut done = pctwin_fullmove::run::DataDone {
        picked,
        ..Default::default()
    };
    let Some(home_path) = home_here() else {
        sink.remove_staging();
        return done;
    };
    let Ok(home) = pctwin_gate::Destination::open(&home_path) else {
        sink.remove_staging();
        return done;
    };
    let names: std::collections::BTreeMap<String, String> = sink
        .arrived
        .iter()
        .filter_map(|(id, a)| Some((id.clone(), a.record.as_ref()?.app.clone())))
        .collect();
    let landed = pctwin_fullmove::data::land_all(
        sink,
        &pctwin_fullmove::data::Landing {
            journal,
            key,
            home: &home,
            home_path: &home_path,
            owner_only: &AppOwnerOnly,
            scope,
            actor: actor.clone(),
            system: pctwin_special::roots::System::here(),
        },
        &names,
    );
    done.landed = landed.iter().map(|l| (l.id.clone(), l.result)).collect();
    done.why = landed
        .iter()
        .filter_map(|l| Some((l.id.clone(), l.why.clone()?)))
        .collect();
    done.effects = landed
        .iter()
        .filter_map(|l| Some((l.app.clone(), l.effect?)))
        .collect();
    done.home = Some(home_path.display().to_string());
    sink.remove_staging();
    done
}

/// At start: removes any app data staging folder an earlier run left (a crash, the laptop turned
/// off part way), so nothing staged, secrets least of all, stays behind.
pub fn sweep_staging<R: Runtime>(app: &AppHandle<R>) {
    if let Some(dir) =
        crate::moving::journal_path(app).and_then(|j| j.parent().map(|p| p.to_path_buf()))
    {
        pctwin_fullmove::data::sweep_staging(&dir);
    }
}

/// A new staging folder for app data, in PCTwin's own data folder.
pub fn data_staging<R: Runtime>(app: &AppHandle<R>) -> Option<std::path::PathBuf> {
    let mut b = [0u8; 8];
    getrandom::fill(&mut b).ok()?;
    let tag: String = b.iter().map(|x| format!("{x:02x}")).collect();
    let dir = crate::moving::journal_path(app)?
        .with_file_name(format!("{}{tag}", pctwin_fullmove::data::STAGING_PREFIX));
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// The old laptop's app data source for the move (`None`: nothing found in time).
pub fn data_source<R: Runtime>(
    app: &AppHandle<R>,
    moving: &crate::moving::AppMoving,
    found: Option<&pctwin_fullmove::extras::SpecialFound>,
) -> Option<pctwin_fullmove::data::Source> {
    let found = found?;
    let key = capture_key()?;
    let mut b = [0u8; 8];
    getrandom::fill(&mut b).ok()?;
    let tag: String = b.iter().map(|x| format!("{x:02x}")).collect();
    let work = std::env::temp_dir().join(format!("pctwin-capture-{tag}"));
    // An app still open: the person is shown which ones to close, and answers Try again or Go
    // on without them (no answer in time is "go on without").
    let (app, moving) = (app.clone(), moving.clone());
    let ask = Box::new(move |apps: &[String]| {
        let answer = moving.wait_for_close();
        let _ = app.emit(
            APPDATA_OPEN_EVENT,
            AppsOpen {
                apps: apps.iter().map(|a| name(a)).collect(),
            },
        );
        answer
            .recv_timeout(Duration::from_secs(600))
            .unwrap_or(false)
    });
    Some(
        pctwin_fullmove::data::Source::new(
            found.planned.clone(),
            key,
            std::sync::Arc::new(pctwin_special::capture::SystemHolders),
            work,
        )
        .asking(ask),
    )
}

/// The old laptop: the person's answer about the apps to close (true: try again).
#[tauri::command]
pub fn move_appdata_answer(
    moving: State<'_, crate::moving::AppMoving>,
    retry: bool,
) -> Result<(), crate::moving::MoveProblem> {
    moving.close_answer(retry)
}

/// An adapter for a laptop with no settings mechanism: nothing is read or written.
struct NoSettings;

impl pctwin_executor::Adapter for NoSettings {
    fn platform(&self) -> pctwin_recipe::Platform {
        pctwin_recipe::Platform {
            os: pctwin_recipe::Os::Windows,
            arch: pctwin_recipe::Arch::X86_64,
        }
    }
    fn install(
        &mut self,
        _: pctwin_grant::InstallGrant,
        _: pctwin_recipe::Catalog,
        _: &str,
    ) -> Result<(), Reason> {
        Err(Reason::RefusedBySafetyCheck)
    }
    fn copy_settings(
        &mut self,
        _: pctwin_grant::SettingGrant,
        _: &pctwin_recipe::SettingsPlace,
        _: &pctwin_recipe::SettingsPlace,
    ) -> Result<(), Reason> {
        Err(Reason::RefusedBySafetyCheck)
    }
    fn apply_setting(
        &mut self,
        _: pctwin_grant::SettingGrant,
        _: &str,
        _: &pctwin_recipe::SettingValue,
    ) -> Result<(), Reason> {
        Err(Reason::RefusedBySafetyCheck)
    }
    fn verify(&mut self, _: &pctwin_recipe::Check) -> bool {
        false
    }
    fn app_presence(
        &mut self,
        _: pctwin_recipe::Catalog,
        _: &str,
    ) -> Result<pctwin_journal::Presence, String> {
        Err("not here".into())
    }
    fn install_running(&mut self, _: pctwin_recipe::Catalog, _: &str) -> Result<bool, String> {
        Err("not here".into())
    }
    fn read_setting(&mut self, _: &str) -> Result<Option<pctwin_recipe::SettingValue>, String> {
        Err("not here".into())
    }
    fn read_settings_files(
        &mut self,
        _: &pctwin_recipe::SettingsPlace,
    ) -> Result<Vec<pctwin_journal::SettingsFile>, String> {
        Err("not here".into())
    }
    fn restore_point(&mut self) -> RestorePoint {
        RestorePoint::NotMade {
            why: "not here".into(),
        }
    }
    fn uninstall(&mut self, _: pctwin_recipe::Catalog, _: &str) -> Result<(), String> {
        Err("not here".into())
    }
    fn write_setting_if(
        &mut self,
        _: &str,
        _: &pctwin_recipe::SettingValue,
        _: Option<&pctwin_recipe::SettingValue>,
    ) -> Result<pctwin_executor::SettingWrite, String> {
        Err("not here".into())
    }
    fn replace_settings_file(
        &mut self,
        _: &pctwin_recipe::SettingsPlace,
        _: &str,
        _: &pctwin_executor::files::Swap<'_>,
    ) -> Result<pctwin_executor::files::Swapped, String> {
        Err("not here".into())
    }
}

/* ---------- Undo (R05, R06) through the flow guard ---------- */

fn plan_line(l: pctwin_fullmove::undo::PlanLine) -> UndoPlanLine {
    UndoPlanLine {
        name: name(&l.name),
        plan: match l.plan {
            pctwin_fullmove::undo::UndoPlan::PutBack => UndoPlan::PutBack,
            pctwin_fullmove::undo::UndoPlan::KeepChanged => UndoPlan::KeepChanged,
            pctwin_fullmove::undo::UndoPlan::RemoveYourself => UndoPlan::RemoveYourself,
        },
    }
}

fn undo_line(l: pctwin_fullmove::undo::Line) -> UndoLine {
    use pctwin_fullmove::undo::Result as R;
    let (result, detail) = match l.result {
        R::Restored => (UndoResult::Restored, None),
        R::AsBefore => (UndoResult::AsBefore, None),
        R::KeptChanged { kept } => (
            UndoResult::KeptChanged,
            Some(format!(
                "{kept} changed after the move, so your newer choice was kept"
            )),
        ),
        R::NotDoneYet { left } => (
            UndoResult::NotDoneYet,
            Some(format!("{left} could not be put back this time")),
        ),
        R::RemoveYourself => (UndoResult::RemoveYourself, None),
    };
    UndoLine {
        name: name(&l.name),
        result,
        detail,
    }
}

const JOB: &str = "move-undo";

/// Where each move's undo book is kept (PCTwin's own data folder, beside the journal), with the
/// one book PCTwin 0.1.x kept moved in first.
fn books_folder<R: Runtime>(app: &AppHandle<R>) -> Option<std::path::PathBuf> {
    let journal = crate::moving::journal_path(app)?;
    let folder = journal.with_file_name(crate::undo_books::FOLDER);
    crate::undo_books::migrate(&folder, &journal.with_file_name(crate::undo_books::LEGACY));
    Some(folder)
}

fn publish<R: Runtime>(
    app: &AppHandle<R>,
    guard: &AppGuard,
    epoch: u64,
    display: DisplayState,
    allowed: Vec<IntentName>,
    progress: Option<Measured>,
    screen: ScreenView,
) {
    let _ = crate::guard::publish(
        app,
        guard,
        Update {
            job: JOB.into(),
            epoch,
            generation: 1,
            display,
            allowed,
            progress,
            screen,
        },
    );
}

/// Opens undo for one move's apps and settings (`id`, the move's own ID from its result; none:
/// the newest move): the preview (R05) through the flow guard, saying which move it is, then,
/// when the person presses Undo there, undo itself (R06). Back closes it. Every other move's
/// book is left as it is.
#[tauri::command]
pub fn fullmove_undo<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppFullMove>,
    guard: State<'_, AppGuard>,
    id: Option<String>,
) -> Result<(), crate::moving::MoveProblem> {
    let held = {
        let books = lock(&state.books);
        match &id {
            Some(id) => books.get(id).cloned(),
            None => None,
        }
    };
    let book = held
        .or_else(|| crate::undo_books::find(&books_folder(&app)?, id.as_deref()))
        .or_else(|| {
            // Not on disk (it could not be written): the newest kept since PCTwin started.
            let books = lock(&state.books);
            match &id {
                Some(_) => None,
                None => books.values().max_by_key(|b| b.when).cloned(),
            }
        })
        .ok_or(crate::moving::MoveProblem::NotMoving)?;
    let epoch = state.epoch.fetch_add(1, Ordering::SeqCst) + 1;
    {
        let view = lock(&guard)
            .begin(JOB, epoch)
            .map_err(|_| crate::moving::MoveProblem::Failed)?;
        let _ = app.emit(crate::guard::EVENT, &view);
    }
    let app2 = app.clone();
    std::thread::Builder::new()
        .name("pctwin-undo".into())
        .spawn(move || run_undo(&app2, epoch, &book))
        .map(|_| ())
        .map_err(|_| crate::moving::MoveProblem::Failed)
}

fn run_undo<R: Runtime>(app: &AppHandle<R>, epoch: u64, book: &UndoBook) {
    let guard = app.state::<AppGuard>();
    let state = app.state::<AppFullMove>();
    let close = || {
        lock(&guard).end();
        let _ = app.emit(UNDO_CLOSED_EVENT, ());
    };
    let journal = crate::moving::journal_path(app).and_then(|p| Journal::open(&p).ok());
    let fake = FullPlan {
        settings: pctwin_settings::generic::FoundPlan::default(),
        ..FullPlan::default()
    };
    let mut adapter = match settings_here(&fake) {
        Some(mut a) => {
            if a.store.remember(book.book.clone()).is_err() {
                close();
                return;
            }
            a
        }
        None => {
            close();
            return;
        }
    };
    let Some(journal) = journal else {
        close();
        return;
    };
    let preview = pctwin_fullmove::undo::preview(&journal, book, &mut adapter)
        .map(|l| l.into_iter().map(plan_line).collect::<Vec<_>>())
        .unwrap_or_default();
    let (lines, more) = bounded(preview);
    publish(
        app,
        &guard,
        epoch,
        DisplayState::AwaitingPerson,
        vec![IntentName::Undo, IntentName::Back],
        None,
        ScreenView::UndoPreview {
            lines,
            more,
            when: book.when,
        },
    );
    // The person's answer: Undo, or Back. Nothing happens without it; an hour without one closes.
    let until = std::time::Instant::now() + Duration::from_secs(3600);
    let mut done = false;
    while std::time::Instant::now() < until {
        let taken = lock(&guard).take();
        for intent in taken {
            if intent.epoch != epoch {
                continue;
            }
            match intent.intent {
                IntentKind::Undo {} if !done => {
                    done = true;
                    publish(
                        app,
                        &guard,
                        epoch,
                        DisplayState::Applying,
                        Vec::new(),
                        Some(Measured {
                            done: 0,
                            total: None,
                            unit: Unit::Items,
                        }),
                        ScreenView::UndoDone {
                            lines: Vec::new(),
                            more: 0,
                        },
                    );
                    let rebuild = Rebuild::new(&journal, &state.keys.install, &state.keys.setting);
                    let home = book
                        .home
                        .as_deref()
                        .and_then(|h| pctwin_gate::Destination::open(std::path::Path::new(h)).ok());
                    let data = home.as_ref().map(|h| (&state.keys.capture, h));
                    let (lines, display) =
                        match pctwin_fullmove::undo::undo(&rebuild, book, &mut adapter, data) {
                            Ok(lines) => (
                                lines.into_iter().map(undo_line).collect::<Vec<_>>(),
                                DisplayState::Complete,
                            ),
                            Err(_) => (Vec::new(), DisplayState::Failed),
                        };
                    let (lines, more) = bounded(lines);
                    publish(
                        app,
                        &guard,
                        epoch,
                        display,
                        vec![IntentName::Back],
                        None,
                        ScreenView::UndoDone { lines, more },
                    );
                }
                IntentKind::Back {} => {
                    close();
                    return;
                }
                _ => {}
            }
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    close();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guard_views::MAX_LINES;
    use pctwin_settings::System;

    /// Gap 1: an app's to-do list reaches the outcome as notes (a restart is the line's own
    /// reason; a registration with nothing in it says nothing).
    #[test]
    fn an_apps_to_do_list_becomes_its_outcome_notes() {
        use pctwin_apps::after::Todo;
        let notes = todo_notes(&[
            Todo::SignIn,
            Todo::LicenceKey,
            Todo::Registration {
                owner: Some("Ada".into()),
                company: None,
                product_id: Some("12345".into()),
            },
            Todo::Registration {
                owner: None,
                company: None,
                product_id: None,
            },
            Todo::Restart,
            Todo::Extras(vec!["Helper".into()]),
        ]);
        assert_eq!(
            notes,
            vec![
                LineNote::SignIn,
                LineNote::LicenceKey,
                LineNote::Registration {
                    owner: Some("Ada".into()),
                    company: None,
                    product_id: Some("12345".into()),
                },
                LineNote::Extras {
                    apps: vec!["Helper".into()],
                },
            ]
        );
    }

    #[test]
    fn the_approved_fingerprint_names_both_the_record_and_the_full_plan() {
        let a = FullPlan::default();
        let fp = approved_fingerprint("ab", &a);
        assert_eq!(fp.len(), 64);
        assert_ne!(fp, approved_fingerprint("ac", &a));
        // The old laptop sent extras: the special categories not looked for are on the plan.
        let b = FullPlan {
            from: Some(System::Windows),
            ..FullPlan::default()
        };
        assert_ne!(fp, approved_fingerprint("ab", &b));
    }

    #[test]
    fn lists_are_bounded_for_the_screens() {
        assert_eq!(MAX_LINES, 200);
    }
}
