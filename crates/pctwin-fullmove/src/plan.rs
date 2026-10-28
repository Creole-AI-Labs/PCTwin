//! The full move's plan on the new laptop, made before the person approves anything: every app
//! with where it comes from or why it needs the person, every setting that moves (grouped by the
//! app or part of the system it belongs to) with whether it adds or replaces a value here, what
//! did not move and why (counted by group), the special categories, and what undo can do for each
//! change. [`FullPlan::digest`] joins the plan the person approves, so an approval of this plan
//! cannot approve another.

use std::collections::BTreeMap;

use serde::Serialize;

use pctwin_apps::routes::{self, AppPlan, Disposition, Here, Hints, Route, Target, WingetBy};
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
    /// PCTwin does not remove apps yet: the person removes it in the system's settings if they
    /// do not want it.
    RemoveYourself,
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
}

/// What a special category's step ends as: a component that would copy is not joined to the
/// move yet, so it is never claimed copied.
pub fn special_reason(step: SpecialStep) -> Option<Reason> {
    match step {
        SpecialStep::SignIn | SpecialStep::Relink => Some(Reason::SignIn),
        SpecialStep::Copy
        | SpecialStep::ExportFirst
        | SpecialStep::NotYet
        | SpecialStep::LookAtIt => Some(Reason::FinishByHand),
    }
}

/// One app and its plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppEntry {
    pub app: FoundApp,
    pub plan: AppPlan,
}

impl AppEntry {
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
}

/// The full move's plan.
#[derive(Debug, Clone, Default)]
pub struct FullPlan {
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
    /// The old laptop's kind of system; `None` when it sent no extras (an older PCTwin).
    pub from: Option<System>,
    /// The extras exactly as they arrived ([`crate::extras::digest`]).
    pub extras_digest: Option<[u8; 32]>,
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

/// The reason an app the plan cannot install itself ends with: the person installs it.
pub fn needs_of(plan: &AppPlan) -> Option<Reason> {
    match plan.disposition {
        Disposition::NeedsAction => Some(Reason::ManualInstall),
        Disposition::Install
            if !plan
                .routes
                .iter()
                .any(|r| matches!(r, Route::Winget { .. })) =>
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
            casks: None,
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
        let Some((x, digest)) = extras else {
            return Self {
                apps,
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
                from: Some(x.system),
                extras_digest: Some(digest),
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
            apps,
            groups,
            not_moved,
            special: x.special.clone(),
            special_searched: x.special_searched,
            special_not_moved: x.special_not_moved,
            from: Some(x.system),
            extras_digest: Some(digest),
            settings,
        }
    }

    /// Every change, in the order the screen shows them: apps, settings, special categories.
    pub fn changes(&self) -> Vec<Change> {
        let mut out = Vec::new();
        for e in &self.apps {
            let (action, undo, needs) = match e.plan.disposition {
                Disposition::AlreadyHere | Disposition::PartOfSystem => {
                    (Action::LeaveAlone, Promise::MustKeep, None)
                }
                _ => match needs_of(&e.plan) {
                    Some(r) => (Action::LeaveAlone, Promise::MustKeep, Some(r)),
                    None => (Action::Install, Promise::RemoveYourself, None),
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
            });
        }
        for s in &self.special {
            let needs = special_reason(s.step);
            out.push(Change {
                title: Title::Name(s.name.clone()),
                kind: Kind::Special,
                action: if needs.is_some() {
                    Action::LeaveAlone
                } else {
                    Action::Merge
                },
                undo: if needs.is_some() {
                    Promise::MustKeep
                } else {
                    Promise::CanReverse
                },
                count: 0,
                needs,
                special: Some(s.step),
                files: s.files,
            });
        }
        // Until the special categories join (2.5), their app data is not moved yet: said, never
        // silent.
        if self.from.is_some() && !self.special_searched {
            out.push(Change {
                title: Title::Key(SPECIAL_NOT_YET),
                kind: Kind::Special,
                action: Action::LeaveAlone,
                undo: Promise::MustKeep,
                count: 0,
                needs: Some(Reason::FinishByHand),
                special: Some(SpecialStep::NotYet),
                files: 0,
            });
        }
        out
    }

    /// The apps PCTwin itself installs, in plan order.
    pub fn to_install(&self) -> Vec<&AppEntry> {
        self.apps
            .iter()
            .filter(|e| e.plan.disposition == Disposition::Install && needs_of(&e.plan).is_none())
            .collect()
    }

    /// Whether the move changes anything here beyond the files (so the administrator prompt and
    /// the restore point are asked for once).
    pub fn changes_anything(&self) -> bool {
        !self.to_install().is_empty() || !self.settings.moves.is_empty()
    }

    /// The digest the approval names: the extras as they arrived and every change shown.
    pub fn digest(&self) -> [u8; 32] {
        digest_of(self.extras_digest.as_ref(), &self.changes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        };
        let plan = FullPlan::build(&record, "1000", None, &here, &mut |_, _| None);
        let names: Vec<_> = plan.apps.iter().map(|e| e.app.name.as_str()).collect();
        assert_eq!(names, ["Writer", "Unsigned Tool", "Already"]);
        let changes = plan.changes();
        assert_eq!(changes[0].action, Action::Install);
        assert_eq!(changes[0].undo, Promise::RemoveYourself);
        assert_eq!(changes[1].needs, Some(Reason::ManualInstall));
        assert_eq!(changes[2].action, Action::LeaveAlone);
        assert_eq!(changes[2].needs, None);
        assert_eq!(plan.to_install().len(), 1);
        // No extras (an older old laptop): no settings, no special line.
        assert!(changes.iter().all(|c| c.kind == Kind::App));
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
        let x = from_discovery(
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
        let installed = AppList::default();
        let none = |_: &FoundApp| vec![];
        let here = NewLaptop {
            system: System::Windows,
            target: target(),
            installed: &installed,
            folders: vec![(FolderRole::Documents, "D:\\Ada\\Documents".into())],
            winget: &none,
        };
        let d = crate::extras::digest(&x.encode());
        let fresh = FullPlan::build(&record, "1000", Some((&x, d)), &here, &mut |_, _| {
            Some(None)
        });
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
