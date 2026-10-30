//! The full move's plan on the new laptop, made before the person approves anything: every app
//! with where it comes from or why it needs the person, every setting that moves (grouped by the
//! app or part of the system it belongs to) with whether it adds or replaces a value here, what
//! did not move and why (counted by group), the special categories, and what undo can do for each
//! change. [`FullPlan::digest`] joins the plan the person approves, so an approval of this plan
//! cannot approve another.

use std::collections::BTreeMap;

use serde::Serialize;

use pctwin_apps::routes::{self, AppPlan, Disposition, Flag, Here, Hints, Route, Target, WingetBy};
use pctwin_recipe::SettingValue;
use pctwin_record::{AppList, FolderRole, FoundApp, FoundWhere, Owner, Record};
use pctwin_settings::System;
use pctwin_settings::catalogue::Catalogue;
use pctwin_settings::classify::{Ctx, FolderMap};
use pctwin_settings::found::Discovery;
use pctwin_settings::generic::{FoundPlan, plan_found};
use pctwin_types::Reason;

use crate::extras::{Extras, NotMovedCount, SpecialLine, SpecialStep, count_not_moved};

/// What a change does to this laptop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Action {
    /// An app is installed.
    Install,
    /// Added beside what is here (the values were not set here).
    Merge,
    /// Something here is replaced; what was here is kept for undo.
    Replace,
    /// Left exactly as it is (already here, or not moved).
    LeaveAlone,
}

/// What undo can do for a change, said before the approval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Promise {
    /// PCTwin puts it back as it was, while nobody changed it since.
    CanReverse,
    /// PCTwin leaves it: it was here before, or it was not changed.
    MustKeep,
    /// PCTwin does not remove this app: the person removes it in the system's settings if they
    /// do not want it.
    RemoveYourself,
    /// An app PCTwin installs on Windows: undo removes it if Windows installed it from a Windows
    /// Installer package and it is still the version PCTwin installed; otherwise the person
    /// removes it in the system's settings (which of the two is known only once it is installed).
    RemovesIfPackage,
    /// An older version here that PCTwin updates if ticked: undo cannot bring the older version
    /// back, so the newer one stays.
    KeepsNewer,
    /// A sign-in, licence or relinking the person does: the app's maker governs it, and undo
    /// cannot take it back.
    NeedsYouMaker,
}

/// What a change is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    App,
    Settings,
    Special,
}

/// A change's title: a name shown exactly as it is, or one of PCTwin's own lines, by its key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
pub enum Title {
    Name(String),
    Key(&'static str),
}

/// The special categories this version does not move yet, as one line.
pub const SPECIAL_NOT_YET: &str = "fullmove-special-not-yet";

/// The old laptop's settings and app data could not be read here, as one line.
pub const EXTRAS_REFUSED: &str = "fullmove-extras-refused";
/// The person chose on the old laptop to leave their apps, settings and app data behind.
pub const LEFT_BY_PERSON: &str = "fullmove-left-by-person";

/// App data places 2.5 could not fully check, as one line.
pub const SPECIAL_NOT_CHECKED: &str = "fullmove-special-not-checked";

/// One change on the approval screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    pub title: Title,
    pub kind: Kind,
    pub action: Action,
    pub undo: Promise,
    /// How many settings it holds (settings only).
    pub count: u64,
    /// Planned as needing the person, and why (it is not done by PCTwin).
    pub needs: Option<Reason>,
    /// A special category's step (special categories only).
    pub special: Option<SpecialStep>,
    /// How many files a special category's component holds.
    pub files: u64,
    /// A secret the person may tick to bring (off unless ticked): its line in the plan's special
    /// list.
    pub secret: Option<u32>,
    /// An older version of this app is on the new laptop and PCTwin can update it if the person
    /// ticks "Update it" (off unless ticked): the app's place in the plan's app list.
    pub update: Option<u32>,
    /// Where PCTwin gets an app it installs: the package's own name in its catalog (winget's ID),
    /// shown beside the old laptop's name for it.
    pub source: Option<String>,
    /// What else the person should know about it, in a word the screens translate.
    pub note: Option<ChangeNote>,
    /// A special category's category, and what the person is told because of it.
    pub category: Option<pctwin_special::category::Category>,
    pub notes: Vec<pctwin_special::category::Note>,

    /// What undo can do if the person ticks "Update it" (only with `update`).
    pub undo_if_updated: Option<Promise>,
    /// What tests of this app between these two systems showed (apps only; `None` for other
    /// changes).
    pub tested: Option<pctwin_apps::compat::Tested>,
}

/// What else the person should know about a change, beside what it does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "note", rename_all = "kebab-case")]
pub enum ChangeNote {
    /// An older version of the app is on this laptop (this one): PCTwin leaves it as it is.
    OlderHere { version: String },
    /// Saved sign-ins to websites or accounts in it are tied to the old laptop: the person signs
    /// in again here.
    Sites,
    /// Built for Intel Macs only, going to an Apple-silicon Mac: it runs through Rosetta, which
    /// Apple is ending (said on its line so the person knows before approving).
    IntelOnly,
}

/// What the person should know about an app beside what happens to it: an older version here
/// first, then that it needs Rosetta.
pub fn app_note(plan: &AppPlan) -> Option<ChangeNote> {
    older_here(plan)
        .map(|version| ChangeNote::OlderHere { version })
        .or_else(|| {
            plan.flags
                .contains(&Flag::IntelOnly)
                .then_some(ChangeNote::IntelOnly)
        })
}

/// The version of an app on this laptop that is older than the old laptop's, if it is.
pub fn older_here(plan: &AppPlan) -> Option<String> {
    plan.flags.iter().find_map(|f| match f {
        Flag::OlderHere { version } => Some(version.clone().unwrap_or_default()),
        _ => None,
    })
}

/// What a special category's step ends as: a component that would copy is not joined to the
/// move yet, so it is never claimed copied.
pub fn special_reason(step: SpecialStep) -> Option<Reason> {
    match step {
        SpecialStep::Copy => None,
        SpecialStep::SignIn | SpecialStep::Relink => Some(Reason::SignIn),
        SpecialStep::ExportFirst | SpecialStep::NotYet | SpecialStep::LookAtIt => {
            Some(Reason::FinishByHand)
        }
    }
}

/// What undo can do for an app PCTwin installs by `plan`'s first route PCTwin installs from:
/// on Windows (winget) undo removes a Windows Installer package it installed, still that version;
/// a Linux package tool's install, and anything else, the person removes (undo lists it).
pub fn install_promise(plan: &AppPlan) -> Promise {
    let first = plan.routes.iter().find(|r| {
        matches!(
            r,
            Route::Winget { .. }
                | Route::Apt { .. }
                | Route::Dnf { .. }
                | Route::Flatpak { .. }
                | Route::Snap { .. }
        )
    });
    match first {
        Some(Route::Winget { .. }) => Promise::RemovesIfPackage,
        _ => Promise::RemoveYourself,
    }
}

/// What undo can do for a special category's step.
pub fn special_promise(step: SpecialStep) -> Promise {
    match step {
        // App data PCTwin places: removed again, each file only while it is what PCTwin wrote.
        SpecialStep::Copy => Promise::CanReverse,
        // The person signs in or relinks: the maker governs it.
        SpecialStep::SignIn | SpecialStep::Relink => Promise::NeedsYouMaker,
        // Nothing PCTwin changes: left as it is.
        SpecialStep::ExportFirst | SpecialStep::NotYet | SpecialStep::LookAtIt => Promise::MustKeep,
    }
}

/// One app and its plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppEntry {
    pub app: FoundApp,
    pub plan: AppPlan,
}

/// What the apps the plan installs need from the internet, before the person approves (Task List
/// 2.7): how many are downloaded. An installer found on the old laptop is never reused yet
/// ([`crate::install`]), so every one is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Downloads {
    pub apps: u64,
}

impl FullPlan {
    /// See [`Downloads`].
    pub fn downloads(&self) -> Downloads {
        downloads_of(&self.apps)
    }
}

/// See [`Downloads`]: only apps PCTwin installs itself from a package it downloads.
pub fn downloads_of(apps: &[AppEntry]) -> Downloads {
    Downloads {
        apps: apps
            .iter()
            .filter(|e| e.plan.disposition == Disposition::Install && e.source().is_some())
            .count() as u64,
    }
}

impl AppEntry {
    /// The package PCTwin itself downloads and installs the app from: its first winget ID or plain
    /// Homebrew cask (the Mac chain, [`crate::mac`]), if any.
    pub fn source(&self) -> Option<&str> {
        self.plan.routes.iter().find_map(|r| match r {
            Route::Winget { id, .. } => Some(id.as_str()),
            Route::Cask { token, .. } => Some(token.as_str()),
            _ => None,
        })
    }

    /// The first route PCTwin itself installs from (a winget package), if any.
    pub fn winget(&self) -> Option<&str> {
        self.plan.routes.iter().find_map(|r| match r {
            Route::Winget { id, .. } => Some(id.as_str()),
            _ => None,
        })
    }
}

/// The settings of one app or part of the system that move, and what they do here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsGroup {
    pub label: String,
    pub count: u64,
    pub action: Action,
}

/// The new laptop, as the plan needs it.
pub struct NewLaptop<'a> {
    pub system: System,
    pub target: Target,
    /// The apps here now, as this laptop's own lists say.
    pub installed: &'a AppList,
    /// Where each of the person's folders is here (for pointing settings at moved files).
    pub folders: Vec<(FolderRole, String)>,
    /// winget IDs suggested for an app, searched as the person (none: winget is not here).
    pub winget: &'a dyn Fn(&FoundApp) -> Vec<(String, WingetBy)>,
    /// Homebrew's cask list, read on a Mac (`None`: not a Mac, or it could not be read).
    pub casks: Option<&'a routes::brew::CaskIndex>,
}

/// Room for the largest installer download held at once (each is removed once installed).
pub const INSTALLER_DOWNLOAD: u64 = 1024 * 1024 * 1024;
/// Room for each app PCTwin installs, once installed.
pub const INSTALLED_APP: u64 = 500 * 1024 * 1024;
/// Room for each file of the app data that copies (a margin: its size is not sent).
pub const APP_DATA_FILE: u64 = 4 * 1024 * 1024;

/// About how much space the move uses beyond its files ([`FullPlan::room_needed`]).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RoomNeeded {
    /// Installer downloads and the apps installed.
    pub installs: u64,
    /// The app data, where it is staged and again where it lands (each).
    pub app_data: u64,
}

/// The full move's plan.
#[derive(Debug, Clone, Default)]
pub struct FullPlan {
    /// The older apps already on the new laptop the person ticked "Update it" for, by their
    /// place in [`Self::apps`]: only these are updated; every other app already here is left.
    pub updates: Vec<u32>,
    /// The mover's apps and everyone's, each with its plan (apps that come with the system are
    /// left out: the new laptop has them).
    pub apps: Vec<AppEntry>,
    pub settings: FoundPlan,
    pub groups: Vec<SettingsGroup>,
    /// Found settings that do not move, counted by group (the old laptop's count and this
    /// laptop's own planning together).
    pub not_moved: Vec<NotMovedCount>,
    pub special: Vec<SpecialLine>,
    /// Whether the old laptop looked for special categories (2.5).
    pub special_searched: bool,
    /// Things 2.5 found there and does not move, counted.
    pub special_not_moved: u64,
    /// Places 2.5 could not fully check there, counted.
    pub special_not_checked: u64,
    /// The old laptop's kind of system; `None` when it sent no extras (an older PCTwin).
    pub from: Option<System>,
    /// The extras exactly as they arrived ([`crate::extras::digest`]).
    pub extras_digest: Option<[u8; 32]>,
    /// The old laptop sent extras this laptop refused (broken, or past a limit): its settings and
    /// app data are not moved, and the plan says so, never silently.
    pub extras_refused: bool,
    /// The person chose on the old laptop to leave apps, settings and app data behind.
    pub left_out_by_person: bool,
    /// Links and junctions among the person's files on the old laptop: never followed, never
    /// moved, and said.
    pub links: u64,
    /// The person's folders on the old laptop, by role, as the old laptop said: where each moved
    /// file was there, so a setting that points at one is pointed at where it really landed
    /// ([`crate::run::DataDone::files`]).
    pub old_folders: Vec<(FolderRole, String)>,
    /// What the old laptop's forgotten-things finder found (2.5).
    pub forgotten: Option<pctwin_special::forgotten::Finding>,

    /// This laptop's kind of system, for the compatibility record (`None`: not known).
    pub to: Option<pctwin_record::AppSystem>,
    /// Each app's state here when the plan was made, by its place in [`Self::apps`] (changes
    /// during the move: [`crate::drift`]).
    pub here_then: Vec<(u32, crate::drift::AppHere)>,
    /// Apps held out of the run because they changed here after approval, by their place in
    /// [`Self::apps`]: never installed until approved again.
    pub held: Vec<u32>,
}

/// What the move needs from the internet (the 2.9 preview).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Internet {
    /// Apps PCTwin downloads and installs (each from its maker's or its store's site; the sizes
    /// are known only once downloaded).
    pub downloads: u64,
    /// Apps and app data the person signs in to again, online.
    pub sign_ins: u64,
}

/// Whether `app` belongs in the mover's plan: theirs, or everyone's.
fn movers(app: &FoundApp, mover: &str) -> bool {
    match &app.owner {
        Owner::Shared => true,
        Owner::Person { account_id } => account_id == mover,
        // The system's own (never the person's to move).
        Owner::System => false,
    }
}

/// The same app on this laptop: by its product code, its package family, its bundle or package,
/// or exactly its name and publisher; never a similar name.
fn same_app(old: &FoundApp, here: &FoundApp) -> bool {
    match (&old.found, &here.found) {
        (
            FoundWhere::WindowsEntry {
                product_code: Some(a),
                ..
            },
            FoundWhere::WindowsEntry {
                product_code: Some(b),
                ..
            },
        ) if a.eq_ignore_ascii_case(b) => true,
        (
            FoundWhere::WindowsPackage { family_name: a, .. },
            FoundWhere::WindowsPackage { family_name: b, .. },
        ) => a == b,
        (
            FoundWhere::MacBundle { bundle_id: a, .. },
            FoundWhere::MacBundle { bundle_id: b, .. },
        ) => a == b,
        (FoundWhere::Deb { package: a }, FoundWhere::Deb { package: b })
        | (FoundWhere::Rpm { package: a }, FoundWhere::Rpm { package: b }) => a == b,
        (FoundWhere::Flatpak { app_id: a, .. }, FoundWhere::Flatpak { app_id: b, .. }) => a == b,
        (FoundWhere::Snap { name: a }, FoundWhere::Snap { name: b }) => a == b,
        _ => {
            old.name == here.name
                && old.publisher.is_some()
                && old.publisher == here.publisher
                && old.system == here.system
        }
    }
}

/// Whether the app is here already, as this laptop's own lists say.
pub fn here_of(app: &FoundApp, installed: &AppList) -> Here {
    match installed.found.iter().find(|h| same_app(app, h)) {
        Some(h) => Here::Present {
            version: h.version.clone(),
        },
        // A list this laptop could not read: it cannot say the app is absent.
        None if !installed.not_read.is_empty() && app.system == installed_system(installed) => {
            Here::Unknown {
                why: "part of this laptop's list of apps could not be read".into(),
            }
        }
        None => Here::Absent,
    }
}

fn installed_system(installed: &AppList) -> pctwin_record::AppSystem {
    installed
        .found
        .first()
        .map(|a| a.system)
        .unwrap_or(pctwin_record::AppSystem::Windows)
}

/// The reason an app the plan cannot install itself ends with: the person installs it. PCTwin
/// installs from winget (Windows, through the administrator helper) and from apt, dnf, Flatpak
/// and Snap (Linux, through the system's own tools: [`crate::unix`]); a Mac's routes are the
/// person's for now.
pub fn needs_of(plan: &AppPlan) -> Option<Reason> {
    match plan.disposition {
        Disposition::NeedsAction => Some(Reason::ManualInstall),
        Disposition::Install
            if !plan.routes.iter().any(|r| {
                matches!(
                    r,
                    Route::Winget { .. }
                        | Route::Apt { .. }
                        | Route::Dnf { .. }
                        | Route::Flatpak { .. }
                        | Route::Snap { .. }
                )
            }) =>
        {
            Some(Reason::ManualInstall)
        }
        _ => None,
    }
}

/// The digest of a change list (bound into the approval).
fn digest_of(extras: Option<&[u8; 32]>, changes: &[Change]) -> [u8; 32] {
    let mut h = blake3::Hasher::new_derive_key("PCTwin full move plan 2026-10-27 v1");
    h.update(extras.map(|d| &d[..]).unwrap_or(&[]));
    h.update(&serde_json::to_vec(changes).unwrap_or_default());
    *h.finalize().as_bytes()
}

impl FullPlan {
    /// The plan for the old laptop's `record` (its apps) and `extras` (as decoded, with the
    /// digest of their bytes), for the person `mover` there, on this laptop. `current` reads a
    /// planned setting's value here now by its executor key (`None`: it could not be read), after
    /// it has been told the plan's places ([`FoundPlan::book`]).
    pub fn build(
        record: &Record,
        mover: &str,
        extras: Option<(&Extras, [u8; 32])>,
        here: &NewLaptop<'_>,
        current: &mut dyn FnMut(&FoundPlan, &str) -> Option<Option<SettingValue>>,
    ) -> Self {
        let rows = routes::rows::shipped();
        let hints = Hints {
            rows,
            winget: here.winget,
            casks: here.casks,
        };
        let apps: Vec<AppEntry> = record
            .apps
            .found
            .iter()
            .filter(|a| movers(a, mover))
            .map(|a| AppEntry {
                app: a.clone(),
                plan: routes::plan(a, here.target, &here_of(a, here.installed), &hints),
            })
            .filter(|e| e.plan.disposition != Disposition::PartOfSystem)
            .collect();
        let here_then = crate::drift::apps_here(&apps, here.installed);
        let Some((x, digest)) = extras else {
            return Self {
                apps,
                to: Some(here.target.system),
                here_then,
                ..Self::default()
            };
        };
        // Folders by role: each old path points at where that folder is here.
        let map = FolderMap(
            x.folders
                .iter()
                .filter_map(|f| {
                    let to = here.folders.iter().find(|(r, _)| *r == f.role)?;
                    Some((f.path.clone(), to.1.clone()))
                })
                .collect(),
        );
        let Ok(cat) = Catalogue::starter() else {
            // Without its catalogue nothing is planned: every setting found is said, not read.
            return Self {
                apps,
                not_moved: vec![NotMovedCount {
                    group: crate::extras::NotMovedGroup::NotRead,
                    count: x.settings.len() as u64,
                }],
                special: x.special.clone(),
                special_searched: x.special_searched,
                special_not_moved: x.special_not_moved,
                special_not_checked: x.special_not_checked,
                from: Some(x.system),
                extras_digest: Some(digest),
                links: x.links,
                forgotten: x.forgotten.clone(),

                to: Some(here.target.system),
                here_then,
                ..Self::default()
            };
        };
        let ctx = Ctx {
            paths: &map,
            same_screens: false,
            hints: Catalogue::starter_hints(),
            // What this laptop says can run, read once per program (warm it at start).
            facts: pctwin_settings::judge::Facts::system(),
        };
        let found = Discovery {
            found: x.settings.clone(),
            not_moved: Vec::new(),
        };
        let settings = plan_found(&cat, x.system, here.system, &found, &ctx);
        let mut not_moved = x.not_moved.clone();
        for n in count_not_moved(&settings.not_moved) {
            match not_moved.iter_mut().find(|m| m.group == n.group) {
                Some(m) => m.count += n.count,
                None => not_moved.push(n),
            }
        }
        not_moved.sort_by_key(|n| n.group);
        let mut groups: BTreeMap<&str, (u64, bool)> = BTreeMap::new();
        for m in &settings.moves {
            let replaces = match current(&settings, &m.key) {
                Some(Some(v)) => v != m.value,
                Some(None) => false,
                // Not readable now: said as replacing, the cautious word.
                None => true,
            };
            let g = groups.entry(m.label.as_str()).or_default();
            g.0 += 1;
            g.1 |= replaces;
        }
        let groups = groups
            .into_iter()
            .map(|(label, (count, replaces))| SettingsGroup {
                label: label.to_string(),
                count,
                action: if replaces {
                    Action::Replace
                } else {
                    Action::Merge
                },
            })
            .collect();
        Self {
            updates: Vec::new(),
            apps,
            groups,
            not_moved,
            special: x.special.clone(),
            special_searched: x.special_searched,
            special_not_moved: x.special_not_moved,
            special_not_checked: x.special_not_checked,
            from: Some(x.system),
            extras_digest: Some(digest),
            settings,
            extras_refused: false,
            left_out_by_person: x.left_out_by_person,
            links: x.links,
            old_folders: x.folders.iter().map(|f| (f.role, f.path.clone())).collect(),
            forgotten: x.forgotten.clone(),

            to: Some(here.target.system),
            here_then,
            held: Vec::new(),
        }
    }

    /// Every change, in the order the screen shows them: apps, settings, special categories.
    pub fn changes(&self) -> Vec<Change> {
        let mut out = Vec::new();
        for (at, e) in self.apps.iter().enumerate() {
            let (action, undo, needs) = match e.plan.disposition {
                Disposition::AlreadyHere | Disposition::PartOfSystem => {
                    (Action::LeaveAlone, Promise::MustKeep, None)
                }
                _ => match needs_of(&e.plan) {
                    Some(r) => (Action::LeaveAlone, Promise::MustKeep, Some(r)),
                    None => (Action::Install, install_promise(&e.plan), None),
                },
            };
            out.push(Change {
                title: Title::Name(e.app.name.clone()),
                kind: Kind::App,
                action,
                undo,
                count: 0,
                needs,
                special: None,
                files: 0,
                category: None,
                notes: Vec::new(),
                secret: None,
                source: (action == Action::Install)
                    .then(|| e.source().map(str::to_string))
                    .flatten(),
                note: app_note(&e.plan),
                undo_if_updated: (older_here(&e.plan).is_some() && e.winget().is_some())
                    .then_some(Promise::KeepsNewer),
                tested: Some(self.tested(&e.app)),
                update: (older_here(&e.plan).is_some() && e.winget().is_some())
                    .then_some(at as u32),
            });
        }
        for g in &self.groups {
            out.push(Change {
                title: Title::Name(g.label.clone()),
                kind: Kind::Settings,
                action: g.action,
                undo: Promise::CanReverse,
                count: g.count,
                needs: None,
                special: None,
                files: 0,
                category: None,
                notes: Vec::new(),
                secret: None,
                update: None,
                source: None,
                note: None,
                undo_if_updated: None,
                tested: None,
            });
        }
        for (at, s) in self.special.iter().enumerate() {
            let needs = special_reason(s.step);
            out.push(Change {
                title: Title::Name(s.name.clone()),
                kind: Kind::Special,
                action: if needs.is_some() {
                    Action::LeaveAlone
                } else {
                    Action::Merge
                },
                undo: special_promise(s.step),
                count: 0,
                needs,
                special: Some(s.step),
                files: s.files,
                category: Some(s.category),
                notes: s.notes.clone(),
                secret: s.secret.then_some(at as u32),
                update: None,
                source: None,
                note: s.sites.then_some(ChangeNote::Sites),
                undo_if_updated: None,
                tested: None,
            });
        }
        // Until the special categories join (2.5), their app data is not moved yet: said, never
        // silent.
        if self.extras_refused {
            out.push(Change {
                title: Title::Key(EXTRAS_REFUSED),
                kind: Kind::Settings,
                action: Action::LeaveAlone,
                undo: Promise::MustKeep,
                count: 0,
                needs: Some(Reason::FinishByHand),
                special: None,
                files: 0,
                category: None,
                notes: Vec::new(),
                secret: None,
                update: None,
                source: None,
                note: None,
                undo_if_updated: None,
                tested: None,
            });
        }
        if self.special_not_checked > 0 {
            out.push(Change {
                title: Title::Key(SPECIAL_NOT_CHECKED),
                kind: Kind::Special,
                action: Action::LeaveAlone,
                undo: Promise::MustKeep,
                count: self.special_not_checked,
                needs: Some(Reason::FinishByHand),
                special: Some(SpecialStep::LookAtIt),
                files: 0,
                category: None,
                notes: Vec::new(),
                secret: None,
                update: None,
                source: None,
                note: None,
                undo_if_updated: None,
                tested: None,
            });
        }
        if self.left_out_by_person {
            out.push(Change {
                title: Title::Key(LEFT_BY_PERSON),
                kind: Kind::Special,
                action: Action::LeaveAlone,
                undo: Promise::MustKeep,
                count: 0,
                needs: None,
                special: None,
                files: 0,
                category: None,
                notes: Vec::new(),
                secret: None,
                update: None,
                source: None,
                note: None,
                undo_if_updated: None,
                tested: None,
            });
        } else if self.from.is_some() && !self.special_searched {
            out.push(Change {
                title: Title::Key(SPECIAL_NOT_YET),
                kind: Kind::Special,
                action: Action::LeaveAlone,
                undo: Promise::MustKeep,
                count: 0,
                needs: Some(Reason::FinishByHand),
                special: Some(SpecialStep::NotYet),
                files: 0,
                category: None,
                notes: Vec::new(),
                secret: None,
                update: None,
                source: None,
                note: None,
                undo_if_updated: None,
                tested: None,
            });
        }
        out
    }

    /// What tests of `app` between the old laptop's system and this one showed.
    pub fn tested(&self, app: &FoundApp) -> pctwin_apps::compat::Tested {
        match self.to {
            Some(to) => pctwin_apps::compat::tested(pctwin_apps::compat::shipped(), app, to),
            None => pctwin_apps::compat::Tested::not_tested(),
        }
    }

    /// What the move needs from the internet: a download for each app PCTwin installs, and a
    /// sign-in for each special category the person signs in to again.
    pub fn internet(&self) -> Internet {
        Internet {
            downloads: self.to_install().len() as u64,
            sign_ins: self
                .special
                .iter()
                .filter(|s| matches!(s.step, SpecialStep::SignIn | SpecialStep::Relink))
                .count() as u64,
        }
    }

    /// The apps PCTwin itself installs, in plan order.
    pub fn to_install(&self) -> Vec<&AppEntry> {
        self.apps
            .iter()
            .enumerate()
            .filter(|(at, _)| !self.held.contains(&(*at as u32)))
            .filter(|(at, e)| {
                (e.plan.disposition == Disposition::Install && needs_of(&e.plan).is_none())
                    // An older app here the person chose to update, by a route PCTwin installs from.
                    || (self.updates.contains(&(*at as u32))
                        && older_here(&e.plan).is_some()
                        && e.winget().is_some())
            })
            .map(|(_, e)| e)
            .collect()
    }

    /// About how much space the move uses on this laptop beyond its files, before it starts
    /// (gap 10a, 28 October 2026). No route carries an installer's size and the old laptop says
    /// only how many files each app's data holds, so both are honest margins, always said with
    /// "about": installing takes room for the largest download held at once
    /// ([`INSTALLER_DOWNLOAD`]) and each app installed ([`INSTALLED_APP`]); the app data that
    /// copies takes room for each of its files ([`APP_DATA_FILE`]) once where it is staged and
    /// once where it lands.
    pub fn room_needed(&self) -> RoomNeeded {
        let apps = self.to_install().len() as u64;
        let installs = if apps == 0 {
            0
        } else {
            INSTALLER_DOWNLOAD.saturating_add(apps.saturating_mul(INSTALLED_APP))
        };
        let files: u64 = self
            .special
            .iter()
            .filter(|s| s.step == SpecialStep::Copy)
            .map(|s| s.files)
            .fold(0, u64::saturating_add);
        RoomNeeded {
            installs,
            app_data: files.saturating_mul(APP_DATA_FILE),
        }
    }

    /// Whether the move changes anything here beyond the files (so the administrator prompt and
    /// the restore point are asked for once).
    pub fn changes_anything(&self) -> bool {
        !self.to_install().is_empty() || !self.settings.moves.is_empty()
    }

    /// The app data to bring: every component that copies, and the secrets at `picked` (lines of
    /// [`FullPlan::special`] the person ticked, one by one).
    pub fn choice(&self, picked: &[u32]) -> crate::data::Choice {
        let mut c = crate::data::Choice::default();
        for (at, s) in self.special.iter().enumerate() {
            if s.step != SpecialStep::Copy {
                continue;
            }
            if s.secret {
                if picked.contains(&(at as u32)) {
                    c.secrets.extend(s.ids.iter().cloned());
                }
            } else {
                c.copy.extend(s.ids.iter().cloned());
            }
        }
        c
    }

    /// The digest the approval names: the extras as they arrived and every change shown.
    pub fn digest(&self) -> [u8; 32] {
        digest_of(self.extras_digest.as_ref(), &self.changes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_intel_only_app_says_rosetta_unless_an_older_version_is_here_and_a_cask_is_its_source() {
        use pctwin_apps::routes::{Card, Disposition, Route};
        let card = Card {
            name: "Painter".into(),
            version: None,
            maker: None,
            web: None,
            licence_key: false,
            sign_in: false,
            registration: vec![],
            why: None,
        };
        let mut p = AppPlan {
            disposition: Disposition::Install,
            routes: vec![
                Route::MacAppStore { adam_id: 1 },
                Route::Cask {
                    token: "painter".into(),
                    version: "1".into(),
                    url: "https://example.com/p.dmg".into(),
                    sha256: None,
                    auto_updates: false,
                    app: "Painter.app".into(),
                },
            ],
            flags: vec![Flag::IntelOnly],
            card,
        };
        assert_eq!(app_note(&p), Some(ChangeNote::IntelOnly));
        p.flags.push(Flag::OlderHere {
            version: Some("0.9".into()),
        });
        assert_eq!(
            app_note(&p),
            Some(ChangeNote::OlderHere {
                version: "0.9".into()
            })
        );
        p.flags.clear();
        assert_eq!(app_note(&p), None);
        assert_eq!(
            serde_json::to_value(ChangeNote::IntelOnly).unwrap(),
            serde_json::json!({ "note": "intel-only" })
        );
        let e = AppEntry {
            app: pctwin_record::FoundApp {
                name: "Painter".into(),
                version: None,
                publisher: None,
                owner: pctwin_record::Owner::Shared,
                system: pctwin_record::AppSystem::Mac,
                found: FoundWhere::MacBundle {
                    bundle_id: "org.example.painter".into(),
                    folder: "Painter.app".into(),
                    app_store_id: Some(1),
                },
                arch: pctwin_record::AppArch::UNKNOWN,
                maker: pctwin_record::AppMaker::Unsigned,
                web: None,
                update_feed: None,
                installers: vec![],
                notes: vec![],
            },
            plan: p,
        };
        assert_eq!(e.source(), Some("painter"));
        assert_eq!(e.winget(), None);
        assert_eq!(downloads_of(std::slice::from_ref(&e)).apps, 1);
    }

    /// Apps, settings and app data the person chose on the old laptop to leave behind are said as
    /// their choice, never as something left to do (first real test, 28 October 2026).
    #[test]
    fn what_the_person_left_behind_is_their_choice_not_a_task() {
        let plan = FullPlan {
            from: Some(System::Windows),
            left_out_by_person: true,
            ..FullPlan::default()
        };
        let changes = plan.changes();
        assert!(
            changes
                .iter()
                .any(|c| c.title == Title::Key(LEFT_BY_PERSON) && c.needs.is_none())
        );
        assert!(
            changes
                .iter()
                .all(|c| c.title != Title::Key(SPECIAL_NOT_YET)),
            "never also 'PCTwin did not move this'"
        );
        let x = Extras::decode(&{
            let mut x = crate::extras::here(Vec::new());
            x.left_out_by_person = true;
            x.encode()
        })
        .unwrap();
        assert!(x.left_out_by_person, "the choice crosses with the extras");
    }
    use crate::extras::{OldFolder, from_discovery};
    use pctwin_record::{AppArch, AppMaker, AppSystem, LaptopId, RegistryView, SignerId};
    use pctwin_settings::found::{Address, FoundSetting, RegKind};

    pub(crate) fn app(name: &str, owner: Owner, maker: AppMaker, code: Option<&str>) -> FoundApp {
        FoundApp {
            name: name.into(),
            version: Some("1.0".into()),
            publisher: Some("Example Ltd".into()),
            owner,
            system: AppSystem::Windows,
            found: FoundWhere::WindowsEntry {
                view: RegistryView::Machine,
                key: name.into(),
                product_code: code.map(str::to_string),
            },
            arch: AppArch::UNKNOWN,
            maker,
            web: None,
            update_feed: None,
            installers: vec![],
            notes: vec![],
        }
    }

    fn signed() -> AppMaker {
        AppMaker::Windows(Box::new(SignerId {
            organization: Some("Example Ltd".into()),
            certificate: [4; 32],
            ..Default::default()
        }))
    }

    fn target() -> Target {
        Target {
            system: AppSystem::Windows,
            arch: AppArch {
                known: true,
                x86: false,
                x64: true,
                arm64: false,
            },
            linux: None,
        }
    }

    fn person(id: &str) -> Owner {
        Owner::Person {
            account_id: id.into(),
        }
    }

    #[test]
    fn every_app_gets_one_line_honestly_and_another_persons_apps_are_not_in_the_movers_plan() {
        let mut record =
            Record::new(LaptopId::from_hex("01010101010101010101010101010101").unwrap());
        record.apps.found = vec![
            app("Writer", Owner::Shared, signed(), Some("{A}")),
            app("Unsigned Tool", person("1000"), AppMaker::Unsigned, None),
            app("Bea's Game", person("2000"), signed(), None),
            app("Already", Owner::Shared, signed(), Some("{B}")),
        ];
        let installed = AppList {
            found: vec![app("Already", Owner::Shared, signed(), Some("{b}"))],
            not_read: vec![],
        };
        let winget = |a: &FoundApp| {
            if a.name == "Writer" {
                vec![("Example.Writer".to_string(), WingetBy::ProductCode)]
            } else {
                vec![]
            }
        };
        let here = NewLaptop {
            system: System::Windows,
            target: target(),
            installed: &installed,
            folders: vec![],
            winget: &winget,
            casks: None,
        };
        let plan = FullPlan::build(&record, "1000", None, &here, &mut |_, _| None);
        let names: Vec<_> = plan.apps.iter().map(|e| e.app.name.as_str()).collect();
        assert_eq!(names, ["Writer", "Unsigned Tool", "Already"]);
        let changes = plan.changes();
        assert_eq!(changes[0].action, Action::Install);
        // A winget install: undo removes it if Windows installed a Windows Installer package.
        assert_eq!(changes[0].undo, Promise::RemovesIfPackage);
        assert_eq!(changes[1].needs, Some(Reason::ManualInstall));
        assert_eq!(changes[2].action, Action::LeaveAlone);
        assert_eq!(changes[2].needs, None);
        assert_eq!(plan.to_install().len(), 1);
        // No extras (an older old laptop): no settings, no special line.
        assert!(changes.iter().all(|c| c.kind == Kind::App));
    }

    /// Gap 6 and U1 (complications audit): an older version already here is left as it is and
    /// said with its version; an app the mover installed for themselves alone on this laptop (in
    /// Windows' per-person list) counts as here, so it is never installed twice.
    #[test]
    fn an_older_version_here_is_said_and_a_per_person_install_counts_as_here() {
        let mut record =
            Record::new(LaptopId::from_hex("01010101010101010101010101010101").unwrap());
        let mut notes = app("Notes", Owner::Shared, signed(), Some("{C}"));
        notes.version = Some("3.0".into());
        record.apps.found = vec![
            notes,
            app("Mine Only", person("1000"), signed(), Some("{D}")),
        ];
        let mut older = app("Notes", Owner::Shared, signed(), Some("{c}"));
        older.version = Some("2.0".into());
        let mut mine = app("Mine Only", person("S-1-5-21-9"), signed(), None);
        mine.found = FoundWhere::WindowsEntry {
            view: RegistryView::Person,
            key: "Mine Only".into(),
            product_code: Some("{d}".into()),
        };
        let installed = AppList {
            found: vec![older, mine],
            not_read: vec![],
        };
        let none = |_: &FoundApp| vec![];
        let here = NewLaptop {
            system: System::Windows,
            target: target(),
            installed: &installed,
            folders: vec![],
            winget: &none,
            casks: None,
        };
        let plan = FullPlan::build(&record, "1000", None, &here, &mut |_, _| None);
        let changes = plan.changes();
        assert_eq!(changes[0].action, Action::LeaveAlone);
        assert_eq!(
            changes[0].note,
            Some(ChangeNote::OlderHere {
                version: "2.0".into()
            })
        );
        assert_eq!(changes[1].action, Action::LeaveAlone);
        assert_eq!(changes[1].note, None);
        assert!(plan.to_install().is_empty(), "nothing installed twice");
        // No route PCTwin installs from: nothing to offer.
        assert_eq!(changes[0].update, None);
        // ...and no word about undo after an update that cannot happen (Task List 2.9).
        assert_eq!(changes[0].undo_if_updated, None);

        // With a winget route, the older app offers "Update it", and only a tick installs it.
        let found = |a: &FoundApp| {
            if a.name == "Notes" {
                vec![("Vendor.Notes".to_string(), WingetBy::ProductCode)]
            } else {
                vec![]
            }
        };
        let here = NewLaptop {
            winget: &found,
            casks: None,
            ..here
        };
        let mut plan = FullPlan::build(&record, "1000", None, &here, &mut |_, _| None);
        let line = plan.changes()[0].update;
        assert_eq!(line, Some(0));
        assert!(
            plan.to_install().is_empty(),
            "never updated without the tick"
        );
        plan.updates = vec![0];
        let names: Vec<&str> = plan
            .to_install()
            .iter()
            .map(|e| e.app.name.as_str())
            .collect();
        assert_eq!(names, ["Notes"]);
        // A tick on an app that is not an older one here installs nothing.
        plan.updates = vec![1];
        assert!(plan.to_install().is_empty());
    }

    fn routes(routes: Vec<Route>) -> AppPlan {
        AppPlan {
            disposition: Disposition::Install,
            routes,
            flags: vec![],
            card: routes::Card {
                name: "X".into(),
                version: None,
                maker: None,
                web: None,
                licence_key: false,
                sign_in: false,
                registration: vec![],
                why: None,
            },
        }
    }

    /// Task List 2.9: what undo can do for an install is said by the route PCTwin installs from:
    /// on Windows (winget) undo removes a Windows Installer package it installed; a Linux package
    /// tool's install is the person's to remove (undo lists it there).
    #[test]
    fn an_install_says_honestly_whether_undo_can_remove_it() {
        let winget = || Route::Winget {
            id: "Example.Writer".into(),
            by: WingetBy::ProductCode,
        };
        let apt = || Route::Apt {
            package: "writer".into(),
        };
        assert_eq!(
            install_promise(&routes(vec![winget()])),
            Promise::RemovesIfPackage
        );
        assert_eq!(
            install_promise(&routes(vec![apt()])),
            Promise::RemoveYourself
        );
        // The first route PCTwin installs from decides, never a later one.
        assert_eq!(
            install_promise(&routes(vec![apt(), winget()])),
            Promise::RemoveYourself
        );
        assert_eq!(
            install_promise(&routes(vec![winget(), apt()])),
            Promise::RemovesIfPackage
        );
        let store = Route::StoreListing {
            family_name: "Example.Writer_abc".into(),
        };
        assert_eq!(
            install_promise(&routes(vec![store, winget()])),
            Promise::RemovesIfPackage
        );
        assert_eq!(install_promise(&routes(vec![])), Promise::RemoveYourself);
    }

    /// Task List 2.9: every special category's step is said with what undo can do: app data PCTwin
    /// places is put back; a sign-in or relinking is the maker's; anything PCTwin does not change
    /// is left.
    #[test]
    fn every_special_step_has_an_honest_undo_explanation() {
        use SpecialStep::*;
        assert_eq!(special_promise(Copy), Promise::CanReverse);
        assert_eq!(special_promise(SignIn), Promise::NeedsYouMaker);
        assert_eq!(special_promise(Relink), Promise::NeedsYouMaker);
        for step in [ExportFirst, NotYet, LookAtIt] {
            assert_eq!(special_promise(step), Promise::MustKeep);
        }
    }

    /// Task List 2.9: every change on the plan carries an undo explanation that fits what it does
    /// (an install is never "kept", nothing left alone is "reversed"), an older app PCTwin may
    /// update says undo keeps the newer one, the preview counts what needs the internet, and each
    /// app carries its compatibility record ("not tested" with no row).
    #[test]
    fn the_preview_explains_undo_for_every_change_and_counts_what_needs_the_internet() {
        let mut record =
            Record::new(LaptopId::from_hex("01010101010101010101010101010101").unwrap());
        let mut notes = app("Notes", Owner::Shared, signed(), Some("{C}"));
        notes.version = Some("3.0".into());
        record.apps.found = vec![
            app("Writer", Owner::Shared, signed(), Some("{A}")),
            notes,
            app("Unsigned Tool", Owner::Shared, AppMaker::Unsigned, None),
        ];
        let mut older = app("Notes", Owner::Shared, signed(), Some("{c}"));
        older.version = Some("2.0".into());
        let installed = AppList {
            found: vec![older],
            not_read: vec![],
        };
        let winget = |a: &FoundApp| match a.name.as_str() {
            "Writer" => vec![("Example.Writer".to_string(), WingetBy::ProductCode)],
            "Notes" => vec![("Example.Notes".to_string(), WingetBy::ProductCode)],
            _ => vec![],
        };
        let here = NewLaptop {
            system: System::Windows,
            target: target(),
            installed: &installed,
            folders: vec![],
            winget: &winget,
            casks: None,
        };
        let mut plan = FullPlan::build(&record, "1000", None, &here, &mut |_, _| None);
        let line = |p: &FullPlan, n: &str| {
            p.changes()
                .into_iter()
                .find(|c| c.title == Title::Name(n.into()))
                .unwrap()
        };
        let writer = line(&plan, "Writer");
        assert_eq!(
            (writer.action, writer.undo),
            (Action::Install, Promise::RemovesIfPackage)
        );
        assert_eq!(writer.undo_if_updated, None);
        let notes = line(&plan, "Notes");
        assert_eq!(
            (notes.action, notes.undo),
            (Action::LeaveAlone, Promise::MustKeep)
        );
        assert_eq!(notes.undo_if_updated, Some(Promise::KeepsNewer));
        let tool = line(&plan, "Unsigned Tool");
        assert_eq!(tool.undo_if_updated, None);
        for c in plan.changes() {
            match c.action {
                Action::Install => assert!(
                    matches!(c.undo, Promise::RemovesIfPackage | Promise::RemoveYourself),
                    "{c:?}"
                ),
                Action::LeaveAlone => assert!(
                    matches!(c.undo, Promise::MustKeep | Promise::NeedsYouMaker),
                    "{c:?}"
                ),
                Action::Merge | Action::Replace => {
                    assert_eq!(c.undo, Promise::CanReverse, "{c:?}")
                }
            }
            // Apps carry their record; with no row each is "not tested".
            if c.kind == Kind::App {
                let t = c.tested.expect("every app has its record");
                assert_eq!(t.outcome, pctwin_apps::compat::Outcome::NotTested);
            } else {
                assert!(c.tested.is_none());
            }
        }
        assert_eq!(plan.to, Some(AppSystem::Windows));
        // One download (Writer) until the person ticks "Update it" for Notes.
        assert_eq!(
            plan.internet(),
            Internet {
                downloads: 1,
                sign_ins: 0
            }
        );
        plan.updates = vec![1];
        assert_eq!(plan.internet().downloads, 2);
        // Sign-ins and relinking count; copies and steps PCTwin does not take do not.
        let special = |name: &str, step| SpecialLine {
            name: name.into(),
            step,
            files: 1,
            secret: false,
            ids: vec![],
            sites: false,
            category: Default::default(),
            notes: Vec::new(),
        };
        plan.special = vec![
            special("Mail", SpecialStep::SignIn),
            special("Browser", SpecialStep::Relink),
            special("Notes data", SpecialStep::Copy),
            special("Game", SpecialStep::ExportFirst),
        ];
        assert_eq!(plan.internet().sign_ins, 2);
        // A plan with no idea of this laptop's system says "not tested", never a row's word.
        plan.to = None;
        let app = plan.apps[0].app.clone();
        assert_eq!(
            plan.tested(&app).outcome,
            pctwin_apps::compat::Outcome::NotTested
        );
    }

    #[test]
    fn settings_are_grouped_said_as_adding_or_replacing_and_the_digest_binds_them() {
        let record = Record::new(LaptopId::from_hex("01010101010101010101010101010101").unwrap());
        let s = |name: &str, v: &str| FoundSetting {
            address: Address::Registry {
                key: "Software\\FakeVendor\\Writer".into(),
                name: name.into(),
                kind: RegKind::Sz,
            },
            label: "FakeVendor Writer".into(),
            value: SettingValue::Text(v.into()),
        };
        let mut x = from_discovery(
            System::Windows,
            Discovery {
                found: vec![s("Theme", "ocean"), s("Font", "serif")],
                not_moved: vec![],
            },
            vec![OldFolder {
                role: FolderRole::Documents,
                path: "C:\\Users\\ada\\Documents".into(),
            }],
        );
        // Gap 8: links and junctions the old laptop counted cross with its extras, to be said.
        x.links = 4;
        let x = Extras::decode(&x.encode()).unwrap();
        let installed = AppList::default();
        let none = |_: &FoundApp| vec![];
        let here = NewLaptop {
            system: System::Windows,
            target: target(),
            installed: &installed,
            folders: vec![(FolderRole::Documents, "D:\\Ada\\Documents".into())],
            winget: &none,
            casks: None,
        };
        let d = crate::extras::digest(&x.encode());
        let fresh = FullPlan::build(&record, "1000", Some((&x, d)), &here, &mut |_, _| {
            Some(None)
        });
        assert_eq!(fresh.links, 4);
        assert_eq!(fresh.groups.len(), 1);
        assert_eq!(fresh.groups[0].count, 2);
        assert_eq!(fresh.groups[0].action, Action::Merge);
        let set = FullPlan::build(&record, "1000", Some((&x, d)), &here, &mut |_, _| {
            Some(Some(SettingValue::Text("forest".into())))
        });
        assert_eq!(set.groups[0].action, Action::Replace);
        assert_ne!(
            fresh.digest(),
            set.digest(),
            "what is shown is what is approved"
        );
        // The special categories not moved yet are said.
        assert!(fresh.changes().iter().any(
            |c| c.title == Title::Key(SPECIAL_NOT_YET) && c.needs == Some(Reason::FinishByHand)
        ));
        assert!(fresh.changes_anything());
    }
}
