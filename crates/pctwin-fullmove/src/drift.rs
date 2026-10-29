//! Changes during the move (Task List 2.9): the rules for what to check again, and hold, when
//! something changes after the person approved the plan.
//!
//! When the plan is made (and so when it is approved: an approval names that very plan) each app's
//! state on this laptop is kept with it (`FullPlan::here_then`: absent, present at a version,
//! unknown), and a destination folder's identity can be kept ([`places_now`]: what it really is on
//! disk, not only its name). Before a step runs, the laptop now is compared with them
//! ([`apps_changed`], [`places_changed`]):
//!
//! - an app that appeared, changed version or went away since: its install step is held (an
//!   install planned for an absent app must not run over one the person installed meanwhile, and
//!   an app left alone because it was here may now be missing);
//! - a destination folder that was moved, removed or replaced by another folder of the same name:
//!   everything landing there is held;
//! - a project or file edited on the old laptop after approval: its app data step is held (the
//!   capture's own check, `pctwin_special::capture::CaptureError::ChangedSinceApproval`, finds
//!   it; [`edited`] turns it into a line here).
//!
//! Only the affected steps are held; everything else carries on. A held step runs only after a
//! fresh approval of the plan as it is now ([`FullPlan::hold`] leaves held apps out of the
//! run until then). Nothing here changes anything.

use std::path::{Path, PathBuf};

use pctwin_apps::routes::Here;
use pctwin_record::AppList;
use serde::{Deserialize, Serialize};

use crate::plan::{AppEntry, FullPlan, here_of};

/// What changed after the approval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Why {
    AppUpdated,
    AppRemoved,
    DestinationChanged,
    ProjectEdited,
}

/// A step held for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Step {
    Install,
    Settings,
    AppData,
    Files,
}

/// One thing that changed, and the steps held for it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Drift {
    pub name: String,
    pub why: Why,
    pub steps: Vec<Step>,
    /// The app's place in the plan's app list (apps only).
    pub app: Option<u32>,
}

/// An app's state here, as the plan saw it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "here", rename_all = "kebab-case")]
pub enum AppHere {
    Absent,
    Present { version: Option<String> },
    Unknown,
}

impl From<&Here> for AppHere {
    fn from(h: &Here) -> Self {
        match h {
            Here::Absent => AppHere::Absent,
            Here::Present { version } => AppHere::Present {
                version: version.clone(),
            },
            Here::Unknown { .. } => AppHere::Unknown,
        }
    }
}

/// What a destination folder really is: its full path with every link resolved, and whether it is
/// a folder (`None`: it is not there).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaceIs {
    pub real: PathBuf,
    /// When it was made, where the system says (a folder removed and made again under the same
    /// name is another folder).
    pub made: Option<u128>,
}

/// What `path` really is now.
pub fn place_is(path: &Path) -> Option<PlaceIs> {
    let m = std::fs::metadata(path).ok()?;
    if !m.is_dir() {
        return None;
    }
    Some(PlaceIs {
        real: std::fs::canonicalize(path).ok()?,
        made: m
            .created()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_nanos()),
    })
}

/// Each app of the plan, by its place in `apps`, as it is on this laptop by `installed`.
pub fn apps_here(apps: &[AppEntry], installed: &AppList) -> Vec<(u32, AppHere)> {
    apps.iter()
        .enumerate()
        .map(|(i, e)| (i as u32, AppHere::from(&here_of(&e.app, installed))))
        .collect()
}

/// Each destination folder, by the name shown, its path and what it is now.
pub type Places = Vec<(String, PathBuf, Option<PlaceIs>)>;

/// The destination folders `places` (each by the name shown and its path) as they are now.
pub fn places_now(places: &[(String, PathBuf)]) -> Places {
    places
        .iter()
        .map(|(n, p)| (n.clone(), p.clone(), place_is(p)))
        .collect()
}

/// The destination folders of `then` that are not what they were: everything landing in each is
/// held.
pub fn places_changed(then: &Places) -> Vec<Drift> {
    then.iter()
        .filter(|(_, path, was)| place_is(path) != *was)
        .map(|(name, _, _)| Drift {
            name: name.clone(),
            why: Why::DestinationChanged,
            steps: vec![Step::Files, Step::AppData],
            app: None,
        })
        .collect()
}

/// The apps of `plan` whose state here changed since the plan was made (`plan.here_then`), with
/// this laptop's apps `installed_now`: only their install steps are held. An app whose state
/// could not be read, then or now, is not called changed (the installer checks it again itself).
pub fn apps_changed(plan: &FullPlan, installed_now: &AppList) -> Vec<Drift> {
    let mut out = Vec::new();
    for (at, was) in &plan.here_then {
        let Some(e) = plan.apps.get(*at as usize) else {
            continue;
        };
        let now = AppHere::from(&here_of(&e.app, installed_now));
        if now == *was || now == AppHere::Unknown || *was == AppHere::Unknown {
            continue;
        }
        out.push(Drift {
            name: e.app.name.clone(),
            why: if now == AppHere::Absent {
                Why::AppRemoved
            } else {
                Why::AppUpdated
            },
            steps: vec![Step::Install],
            app: Some(*at),
        });
    }
    out
}

/// The line for app data `name` whose files changed on the old laptop after approval (found by
/// the capture's own check).
pub fn edited(name: &str) -> Drift {
    Drift {
        name: name.to_string(),
        why: Why::ProjectEdited,
        steps: vec![Step::AppData],
        app: None,
    }
}

/// The app data lines of `plan` that changed on the old laptop after approval, with
/// `changed(id)` telling which components the old laptop refused for it (the capture's own check,
/// [`crate::data::Sink::changed_after_approval`]): each line once. Their app data is not placed
/// (nothing arrived for them); they are asked about again.
pub fn data_changed(plan: &FullPlan, changed: impl Fn(&str) -> bool) -> Vec<Drift> {
    plan.special
        .iter()
        .filter(|s| s.ids.iter().any(|id| changed(id)))
        .map(|s| edited(&s.name))
        .collect()
}

/// Every piece of app data on a line where any piece was edited after the approval (`changed`):
/// none of that line is placed, so the line is whole or not there at all.
pub fn data_held(
    plan: &FullPlan,
    changed: impl Fn(&str) -> bool,
) -> std::collections::BTreeSet<String> {
    plan.special
        .iter()
        .filter(|s| s.ids.iter().any(|id| changed(id)))
        .flat_map(|s| s.ids.iter().cloned())
        .collect()
}

impl FullPlan {
    /// Leaves the apps `drift` holds out of the run until a fresh approval
    /// ([`FullPlan::to_install`] skips them; the run ends each needs-action).
    pub fn hold(&mut self, drift: &[Drift]) {
        self.held.extend(drift.iter().filter_map(|d| d.app));
        self.held.sort_unstable();
        self.held.dedup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{Action, NewLaptop};
    use pctwin_apps::routes::{Target, WingetBy};
    use pctwin_record::{
        AppArch, AppMaker, AppSystem, FoundApp, FoundWhere, LaptopId, Owner, Record, RegistryView,
        SignerId,
    };
    use pctwin_settings::System;
    use pctwin_types::{ItemStatus, Reason};

    fn app(name: &str, code: &str, version: &str) -> FoundApp {
        FoundApp {
            name: name.into(),
            version: Some(version.into()),
            publisher: Some("Example Ltd".into()),
            owner: Owner::Shared,
            system: AppSystem::Windows,
            found: FoundWhere::WindowsEntry {
                view: RegistryView::Machine,
                key: name.into(),
                product_code: Some(code.into()),
            },
            arch: AppArch::UNKNOWN,
            maker: AppMaker::Windows(Box::new(SignerId {
                organization: Some("Example Ltd".into()),
                certificate: [4; 32],
                ..Default::default()
            })),
            web: None,
            update_feed: None,
            installers: vec![],
            notes: vec![],
        }
    }

    fn list(apps: Vec<FoundApp>) -> AppList {
        AppList {
            found: apps,
            not_read: vec![],
        }
    }

    fn plan(installed: &AppList) -> FullPlan {
        let mut record =
            Record::new(LaptopId::from_hex("01010101010101010101010101010101").unwrap());
        record.apps.found = vec![
            app("Writer", "{A}", "3.0"),
            app("Notes", "{B}", "2.0"),
            app("Paint", "{C}", "1.0"),
        ];
        let winget = |a: &FoundApp| vec![(format!("Example.{}", a.name), WingetBy::ProductCode)];
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
            installed,
            folders: vec![],
            winget: &winget,
        };
        FullPlan::build(&record, "1000", None, &here, &mut |_, _| None)
    }

    /// Task List 2.9: an app that was installed, updated or removed on this laptop after the plan
    /// was approved holds only its own install step; nothing else is held, and an unchanged
    /// laptop holds nothing.
    #[test]
    fn only_the_apps_that_changed_after_approval_are_held() {
        let then = list(vec![app("Notes", "{B}", "2.0"), app("Paint", "{C}", "1.0")]);
        let mut p = plan(&then);
        assert!(apps_changed(&p, &then).is_empty(), "nothing changed");
        // Writer was installed by the person meanwhile; Notes was updated; Paint was removed.
        let now = list(vec![
            app("Writer", "{A}", "3.0"),
            app("Notes", "{B}", "2.5"),
        ]);
        let drift = apps_changed(&p, &now);
        let words: Vec<(&str, Why)> = drift.iter().map(|d| (d.name.as_str(), d.why)).collect();
        assert_eq!(
            words,
            [
                ("Writer", Why::AppUpdated),
                ("Notes", Why::AppUpdated),
                ("Paint", Why::AppRemoved)
            ]
        );
        assert!(drift.iter().all(|d| d.steps == [Step::Install]));
        assert_eq!(drift[0].app, Some(0));
        // Held: Writer is no longer installed by the run; the others were never to be.
        assert_eq!(p.to_install().len(), 1);
        p.hold(&drift[..1]);
        assert!(p.to_install().is_empty());
        p.hold(&drift);
        assert_eq!(p.held, [0, 1, 2], "each once, in order");
        // A list that could not be read now says nothing changed (the installer checks again).
        let unread = AppList {
            found: vec![],
            not_read: vec![pctwin_record::AppsNotRead {
                owner: Owner::Shared,
                place: pctwin_record::AppPlace::WindowsMachineList,
                why: "could not be read".into(),
            }],
        };
        let p = plan(&then);
        assert!(apps_changed(&p, &unread).is_empty());
        // Holding changes no line the person approved.
        let q = plan(&then);
        assert_eq!(p.digest(), q.digest());
        assert_eq!(p.changes()[0].action, Action::Install);
    }

    /// A held app ends needs-action in the run, never installed and never said to be here.
    #[test]
    fn a_held_app_is_never_installed_or_said_to_be_here() {
        let then = list(vec![app("Notes", "{B}", "2.0")]);
        let mut p = plan(&then);
        p.hold(&apps_changed(&p, &list(vec![app("Writer", "{A}", "3.0")])));
        assert_eq!(p.held, [0, 1]);
        let r = crate::run::app_lines(&p, &p.to_install(), Vec::new()).0;
        let writer = r.iter().find(|(n, _)| n == "Writer").unwrap();
        assert_eq!(writer.1.result.status(), ItemStatus::NeedsAction);
        assert_eq!(writer.1.result.reason(), Some(Reason::FinishByHand));
        let notes = r.iter().find(|(n, _)| n == "Notes").unwrap();
        assert_eq!(notes.1.result.status(), ItemStatus::NeedsAction);
    }

    #[test]
    fn a_destination_replaced_by_another_folder_of_the_same_name_is_held() {
        let t = tempfile::tempdir().unwrap();
        let pics = t.path().join("Pictures");
        std::fs::create_dir(&pics).unwrap();
        let then = places_now(&[("Pictures".into(), pics.clone())]);
        assert!(then[0].2.is_some());
        assert!(places_changed(&then).is_empty());
        std::fs::remove_dir(&pics).unwrap();
        let gone = places_changed(&then);
        assert_eq!(gone.len(), 1);
        assert_eq!(gone[0].why, Why::DestinationChanged);
        assert_eq!(gone[0].steps, [Step::Files, Step::AppData]);
        // Made again under the same name: another folder (when the system keeps creation times).
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::create_dir(&pics).unwrap();
        let again = places_changed(&then);
        if then[0].2.as_ref().is_some_and(|p| p.made.is_some()) {
            assert_eq!(
                again.len(),
                1,
                "a new folder of the same name is not the approved one"
            );
        }
        // A file where the folder was is not a folder.
        std::fs::remove_dir(&pics).unwrap();
        std::fs::write(&pics, b"x").unwrap();
        assert_eq!(place_is(&pics), None);
        assert_eq!(edited("Thesis.svg").why, Why::ProjectEdited);
        assert_eq!(edited("Thesis.svg").steps, [Step::AppData]);
    }

    /// Fresh-context review F5: what the old laptop refused because it changed after approval
    /// becomes one line per app, holding its app data; nothing else does.
    #[test]
    fn app_data_edited_after_approval_holds_its_line() {
        use crate::data::{Arrived, Choice, NotCaptured, Sink};
        use crate::extras::{SpecialLine, SpecialStep};
        let special = |name: &str, ids: &[&str]| SpecialLine {
            name: name.into(),
            step: SpecialStep::Copy,
            files: 1,
            secret: false,
            ids: ids.iter().map(|i| i.to_string()).collect(),
            sites: false,
        };
        let plan = FullPlan {
            special: vec![
                special("Mail", &["mail-1", "mail-2"]),
                special("Notes", &["notes-1"]),
                special("Photos", &["photos-1"]),
            ],
            ..FullPlan::default()
        };
        let t = tempfile::tempdir().unwrap();
        let mut sink = Sink::new(
            Choice {
                copy: vec!["mail-1".into(), "mail-2".into(), "notes-1".into()],
                secrets: vec![],
            },
            t.path().to_path_buf(),
        );
        let not = |why| Arrived {
            not_captured: Some(why),
            ..Arrived::default()
        };
        sink.arrived
            .insert("mail-2".into(), not(NotCaptured::Changing));
        sink.arrived
            .insert("notes-1".into(), not(NotCaptured::AppOpen));
        sink.arrived.insert("photos-1".into(), Arrived::default());
        assert!(sink.changed_after_approval("mail-2"));
        assert!(!sink.changed_after_approval("notes-1"));
        assert!(!sink.changed_after_approval("photos-1"));
        assert!(!sink.changed_after_approval("mail-1"));
        let d = data_changed(&plan, |id| sink.changed_after_approval(id));
        assert_eq!(d, [edited("Mail")]);
        // The piece that did not change is held with it: the line is placed whole or not at all.
        let held = data_held(&plan, |id| sink.changed_after_approval(id));
        assert_eq!(held, ["mail-1".to_string(), "mail-2".to_string()].into());
    }
}
