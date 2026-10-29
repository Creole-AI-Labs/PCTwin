//! Repair only what failed (Task List 2.9; research RF10): "Fix these 3 items" in the finishing
//! flow. Only the items that did not finish are tried again; every successful result is kept as
//! it was; what each retry needs is checked again first.
//!
//! The last rebuild of apps, settings and app data on this laptop is kept here ([`remember`]):
//! the plan as it ran, and every line's result. "Fix these" ([`repair_run`]) builds a part of
//! that plan holding only the picked unfinished items (every other app held, every other group
//! of settings left out, no app data), checks again what the retry needs (no other step going
//! on; the system's app installer is here), and runs it through the same rebuild as the move
//! (the administrator helper, the journal, its own undo record). Each retried line takes its new
//! result; every other line, finished or not, is kept exactly as it was.
//!
//! Not retried here, and said: an app the person installs themselves (their step, not a
//! failure), an app held because it changed after approval (that needs a fresh approval first:
//! [`super::changes`]), and app data (it comes from the old laptop again: `old-laptop`).

use std::sync::{Mutex, MutexGuard, OnceLock};

use pctwin_fullmove::drift::Drift;
use pctwin_fullmove::plan::{FullPlan, Kind, Title, needs_of};
use pctwin_types::{ItemStatus, Reason};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Runtime};

use crate::fullmove::RebuiltLine;
use crate::guard_views::{bounded, name};

pctwin_types::listed_enum! {
    /// What a retry needs, checked again before it runs.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum Prerequisite {
        /// The old laptop, paired and connected (for files and app data from it).
        OldLaptop,
        /// The internet (for an app's download).
        Internet,
        /// The system's administrator prompt, answered yes (for installing an app).
        Administrator,
        /// The system's own installer tool (winget, a Linux package tool).
        InstallerTool,
        /// Room on this laptop.
        Space,
        /// The app closed (for its data or settings).
        CloseApp,
    }
}

/// One item that did not finish.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairLine {
    /// Its place in the offer (what the screens send back).
    pub line: u32,
    pub title: Title,
    pub kind: Kind,
    pub status: ItemStatus,
    pub reason: Option<Reason>,
    /// What a retry needs that is not there now (empty: it can be tried now).
    pub waits_for: Vec<Prerequisite>,
}

/// What "Fix these" can try again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairOffer {
    /// The offer's version: a retry of another version is refused.
    pub generation: u64,
    pub lines: Vec<RepairLine>,
    pub more: u64,
    /// Results that finished and are kept as they are.
    pub kept: u64,
}

/// What a retry did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairDone {
    pub generation: u64,
    /// Items that finished this time.
    pub fixed: u64,
    /// Items still not finished, each with what it waits for.
    pub lines: Vec<RepairLine>,
    pub more: u64,
    /// Results that finished before, kept as they were.
    pub kept: u64,
    /// The retry's own undo record (`None`: it changed nothing undo could put back), for
    /// `fullmove_undo`.
    pub undo_id: Option<String>,
}

pctwin_types::listed_enum! {
    /// Why a retry did not start.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum RepairProblem {
        /// Nothing picked is unfinished, or can be tried now.
        NothingToRepair,
        /// The offer changed since the screens saw it.
        Stale,
        /// A move, an undo or another retry is going on.
        Busy,
        Failed,
    }
}

/* ---------- The last rebuild, kept ---------- */

/// The last rebuild on this laptop.
#[derive(Debug, Clone)]
pub struct Last {
    /// The plan as it ran (apps held for changes after approval in [`FullPlan::held`]).
    pub plan: FullPlan,
    pub me: String,
    /// What landed where (for settings that point at moved files).
    pub files: Vec<(String, String)>,
    /// Every line's result, in plan order.
    pub lines: Vec<RebuiltLine>,
    /// What changed after approval and waits for the person.
    pub drift: Vec<Drift>,
}

/// [`Last`], with the versions the screens answer for.
#[derive(Debug, Clone)]
pub struct Kept {
    pub generation: u64,
    pub last: Last,
}

fn kept() -> MutexGuard<'static, Option<Kept>> {
    static KEPT: OnceLock<Mutex<Option<Kept>>> = OnceLock::new();
    KEPT.get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Runs `f` on the kept rebuild.
pub fn with_kept<T>(f: impl FnOnce(&mut Option<Kept>) -> T) -> T {
    f(&mut kept())
}

/// Keeps `last` as the last rebuild, and tells the screens of anything that changed after
/// approval.
pub fn remember<R: Runtime>(app: &AppHandle<R>, last: Last) {
    let view = with_kept(|k| {
        let generation = k.as_ref().map_or(1, |k| k.generation.wrapping_add(1));
        let kept = Kept { generation, last };
        let view = super::changes::view(&kept);
        *k = Some(kept);
        view
    });
    if let Some(v) = view {
        let _ = app.emit(super::changes::EVENT, v);
    }
}

/* ---------- The rules ---------- */

/// What a retry of `line` waits for, with `installs` (this laptop installs apps itself); `None`
/// when it is not one to retry here (finished, the person's own step or choice, or held for a
/// fresh approval).
pub fn retry_of(plan: &FullPlan, line: &RebuiltLine, installs: bool) -> Option<Vec<Prerequisite>> {
    if !matches!(line.status, ItemStatus::Failed | ItemStatus::NeedsAction) {
        return None;
    }
    match line.kind {
        Kind::App => {
            let (at, e) = plan
                .apps
                .iter()
                .enumerate()
                .find(|(_, e)| Title::Name(name(&e.app.name)) == line.title)?;
            if plan.held.contains(&(at as u32)) || needs_of(&e.plan).is_some() {
                return None;
            }
            // A sign-in or licence left to do after a good install is the person's step.
            if line.status == ItemStatus::NeedsAction
                && matches!(
                    line.reason,
                    Some(Reason::SignIn | Reason::Activation | Reason::Restart)
                )
            {
                return None;
            }
            Some(if installs {
                Vec::new()
            } else {
                vec![Prerequisite::InstallerTool]
            })
        }
        Kind::Settings => match &line.title {
            // PCTwin's own lines (not a group of settings) have nothing to retry.
            Title::Key(_) => None,
            Title::Name(_) => Some(Vec::new()),
        },
        // App data comes from the old laptop again.
        Kind::Special => match &line.title {
            Title::Key(_) => None,
            Title::Name(_) => Some(vec![Prerequisite::OldLaptop]),
        },
    }
}

/// The lines of `last` a retry is offered for, each with what it waits for.
pub fn offered(last: &Last, installs: bool) -> Vec<(usize, Vec<Prerequisite>)> {
    last.lines
        .iter()
        .enumerate()
        .filter_map(|(i, l)| Some((i, retry_of(&last.plan, l, installs)?)))
        .collect()
}

fn line_of(n: usize, l: &RebuiltLine, waits_for: Vec<Prerequisite>) -> RepairLine {
    RepairLine {
        line: n as u32,
        title: l.title.clone(),
        kind: l.kind,
        status: l.status,
        reason: l.reason,
        waits_for,
    }
}

fn finished(l: &RebuiltLine) -> bool {
    matches!(
        l.status,
        ItemStatus::Copied | ItemStatus::Reinstalled | ItemStatus::Converted
    )
}

/// The offer for `k`.
pub fn offer(k: &Kept, installs: bool) -> RepairOffer {
    let lines: Vec<RepairLine> = offered(&k.last, installs)
        .into_iter()
        .enumerate()
        .map(|(n, (i, w))| line_of(n, &k.last.lines[i], w))
        .collect();
    let (lines, more) = bounded(lines);
    RepairOffer {
        generation: k.generation,
        lines,
        more,
        kept: k.last.lines.iter().filter(|l| finished(l)).count() as u64,
    }
}

/// The part of `plan` that retries exactly `picked` (lines of the last rebuild): only those apps
/// can install (every other app held), only those groups of settings move, and no app data.
pub fn part_plan(plan: &FullPlan, picked: &[&RebuiltLine]) -> FullPlan {
    let wants = |kind: Kind, t: &Title| picked.iter().any(|l| l.kind == kind && l.title == *t);
    let mut part = plan.clone();
    for (at, e) in plan.apps.iter().enumerate() {
        if !wants(Kind::App, &Title::Name(name(&e.app.name))) {
            part.held.push(at as u32);
        }
    }
    part.held.sort_unstable();
    part.held.dedup();
    let groups: Vec<String> = plan
        .groups
        .iter()
        .filter(|g| wants(Kind::Settings, &Title::Name(name(&g.label))))
        .map(|g| g.label.clone())
        .collect();
    part.groups.retain(|g| groups.contains(&g.label));
    part.settings.moves.retain(|m| groups.contains(&m.label));
    part.special.clear();
    part.special_not_checked = 0;
    part.special_not_moved = 0;
    part.extras_refused = false;
    part
}

/// Puts each new line of a retry over the line it retried, if it was `picked`; every other line
/// is kept exactly as it was. The number of picked lines that finished this time.
pub fn merge(lines: &mut [RebuiltLine], new: &[RebuiltLine], picked: &[(Kind, Title)]) -> u64 {
    let mut fixed = 0;
    for n in new {
        if !picked.iter().any(|(k, t)| *k == n.kind && *t == n.title) {
            continue;
        }
        if let Some(old) = lines
            .iter_mut()
            .find(|l| l.kind == n.kind && l.title == n.title)
        {
            if !finished(old) && finished(n) {
                fixed += 1;
            }
            *old = n.clone();
        }
    }
    fixed
}

/* ---------- The commands ---------- */

/// Runs `part` through the same rebuild as the move, with no check for changes (the retry was
/// just asked for): every line, and the retry's undo record.
pub fn run_part<R: Runtime>(
    app: &AppHandle<R>,
    part: &FullPlan,
    me: &str,
    files: Vec<(String, String)>,
) -> Result<(Vec<RebuiltLine>, Option<String>), RepairProblem> {
    let journal = crate::moving::journal_path(app)
        .and_then(|p| pctwin_journal::Journal::open(&p).ok())
        .ok_or(RepairProblem::Failed)?;
    let (view, lines) = crate::fullmove::rebuild_full(app, part, &journal, me, None, files);
    Ok((lines, view.undo_id))
}

/// Whether nothing else is going on (no move, no rebuild, no undo).
fn free() -> bool {
    super::safe_stop::gate().view().safe
}

/// What "Fix these" can try again for the last move.
#[tauri::command]
pub fn repair_offer() -> RepairOffer {
    let installs = crate::fullmove::installs_here();
    with_kept(|k| match k {
        Some(k) => offer(k, installs),
        None => RepairOffer {
            generation: 0,
            lines: Vec::new(),
            more: 0,
            kept: 0,
        },
    })
}

/// Tries again exactly the picked lines of the offer `generation` that can be tried now.
#[tauri::command]
pub async fn repair_run<R: Runtime>(
    app: AppHandle<R>,
    generation: u64,
    lines: Vec<u32>,
) -> Result<RepairDone, RepairProblem> {
    tauri::async_runtime::spawn_blocking(move || run(&app, generation, &lines))
        .await
        .map_err(|_| RepairProblem::Failed)?
}

fn run<R: Runtime>(
    app: &AppHandle<R>,
    generation: u64,
    picked: &[u32],
) -> Result<RepairDone, RepairProblem> {
    if !free() {
        return Err(RepairProblem::Busy);
    }
    // The prerequisites, checked again now.
    let installs = crate::fullmove::installs_here();
    let (part, me, files, wanted) = with_kept(|k| {
        let k = k.as_ref().ok_or(RepairProblem::NothingToRepair)?;
        if k.generation != generation {
            return Err(RepairProblem::Stale);
        }
        let offered = offered(&k.last, installs);
        let chosen: Vec<&RebuiltLine> = picked
            .iter()
            .filter_map(|n| offered.get(*n as usize))
            .filter(|(_, waits)| waits.is_empty())
            .map(|(i, _)| &k.last.lines[*i])
            .collect();
        if chosen.is_empty() {
            return Err(RepairProblem::NothingToRepair);
        }
        let wanted: Vec<(Kind, Title)> = chosen.iter().map(|l| (l.kind, l.title.clone())).collect();
        Ok((
            part_plan(&k.last.plan, &chosen),
            k.last.me.clone(),
            k.last.files.clone(),
            wanted,
        ))
    })?;
    let (new, undo_id) = run_part(app, &part, &me, files)?;
    with_kept(|k| {
        let k = k.as_mut().ok_or(RepairProblem::Failed)?;
        let fixed = merge(&mut k.last.lines, &new, &wanted);
        k.generation = k.generation.wrapping_add(1);
        let o = offer(k, installs);
        Ok(RepairDone {
            generation: o.generation,
            fixed,
            lines: o.lines,
            more: o.more,
            kept: o.kept,
            undo_id,
        })
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use pctwin_fullmove::plan::SettingsGroup;

    pub(crate) fn line(
        kind: Kind,
        title: &str,
        status: ItemStatus,
        reason: Option<Reason>,
    ) -> RebuiltLine {
        RebuiltLine {
            title: Title::Name(title.into()),
            kind,
            count: 0,
            status,
            reason,
            notes: Vec::new(),
        }
    }

    pub(crate) fn plan_with(apps: &[(&str, bool)]) -> FullPlan {
        use pctwin_apps::routes::{AppPlan, Card, Disposition, Route, WingetBy};
        use pctwin_record::{
            AppArch, AppMaker, AppSystem, FoundApp, FoundWhere, Owner, RegistryView,
        };
        let entry = |n: &str, ours: bool| pctwin_fullmove::plan::AppEntry {
            app: FoundApp {
                name: n.into(),
                version: None,
                publisher: None,
                owner: Owner::Shared,
                system: AppSystem::Windows,
                found: FoundWhere::WindowsEntry {
                    view: RegistryView::Machine,
                    key: n.into(),
                    product_code: Some(format!("{{{n}}}")),
                },
                arch: AppArch::UNKNOWN,
                maker: AppMaker::Unsigned,
                web: None,
                update_feed: None,
                installers: vec![],
                notes: vec![],
            },
            plan: AppPlan {
                disposition: if ours {
                    Disposition::Install
                } else {
                    Disposition::NeedsAction
                },
                routes: if ours {
                    vec![Route::Winget {
                        id: format!("Example.{n}"),
                        by: WingetBy::ProductCode,
                    }]
                } else {
                    vec![]
                },
                flags: vec![],
                card: Card {
                    name: n.into(),
                    version: None,
                    maker: None,
                    web: None,
                    licence_key: false,
                    sign_in: false,
                    registration: vec![],
                    why: None,
                },
            },
        };
        FullPlan {
            apps: apps.iter().map(|(n, ours)| entry(n, *ours)).collect(),
            groups: vec![
                SettingsGroup {
                    label: "Writer settings".into(),
                    count: 3,
                    action: pctwin_fullmove::plan::Action::Replace,
                },
                SettingsGroup {
                    label: "Desktop".into(),
                    count: 2,
                    action: pctwin_fullmove::plan::Action::Merge,
                },
            ],
            ..FullPlan::default()
        }
    }

    fn last() -> Last {
        use ItemStatus::*;
        Last {
            plan: plan_with(&[
                ("Writer", true),
                ("Viewer", true),
                ("Sketch", false),
                ("Held", true),
            ]),
            me: "1001".into(),
            files: vec![],
            lines: vec![
                line(Kind::App, "Writer", Failed, Some(Reason::InstallFailed)),
                line(Kind::App, "Viewer", Reinstalled, None),
                line(
                    Kind::App,
                    "Sketch",
                    NeedsAction,
                    Some(Reason::ManualInstall),
                ),
                line(Kind::App, "Held", NeedsAction, Some(Reason::FinishByHand)),
                line(
                    Kind::Settings,
                    "Writer settings",
                    Failed,
                    Some(Reason::NotVerified),
                ),
                line(Kind::Settings, "Desktop", Copied, None),
                line(
                    Kind::Special,
                    "Mail",
                    NeedsAction,
                    Some(Reason::NotVerified),
                ),
                RebuiltLine {
                    title: Title::Key(pctwin_fullmove::plan::EXTRAS_REFUSED),
                    ..line(Kind::Settings, "", NeedsAction, Some(Reason::FinishByHand))
                },
                RebuiltLine {
                    title: Title::Key(pctwin_fullmove::plan::SPECIAL_NOT_YET),
                    ..line(Kind::Special, "", NeedsAction, Some(Reason::FinishByHand))
                },
            ],
            drift: vec![],
        }
    }

    /// Task List 2.9 (RF10): only what did not finish is offered; the person's own steps, a held
    /// app and PCTwin's own lines are not; app data says it needs the old laptop; finished results
    /// are counted as kept.
    #[test]
    fn only_what_failed_is_offered_with_what_it_waits_for() {
        let mut l = last();
        l.plan.held = vec![3];
        let k = Kept {
            generation: 4,
            last: l,
        };
        let o = offer(&k, true);
        let names: Vec<(&Title, &[Prerequisite])> = o
            .lines
            .iter()
            .map(|l| (&l.title, l.waits_for.as_slice()))
            .collect();
        assert_eq!(
            names,
            [
                (&Title::Name("Writer".into()), &[][..]),
                (&Title::Name("Writer settings".into()), &[][..]),
                (&Title::Name("Mail".into()), &[Prerequisite::OldLaptop][..]),
            ]
        );
        assert_eq!(
            o.lines.iter().map(|l| l.line).collect::<Vec<_>>(),
            [0, 1, 2]
        );
        assert_eq!(o.kept, 2);
        assert_eq!(o.generation, 4);
        // With no installer here, the app waits for one.
        let o = offer(&k, false);
        assert_eq!(o.lines[0].waits_for, [Prerequisite::InstallerTool]);
        // A good install with a sign-in left is the person's step.
        let mut l = last();
        l.lines[0] = line(
            Kind::App,
            "Writer",
            ItemStatus::NeedsAction,
            Some(Reason::SignIn),
        );
        assert!(retry_of(&l.plan, &l.lines[0], true).is_none());
        l.lines[0] = line(
            Kind::App,
            "Writer",
            ItemStatus::NeedsAction,
            Some(Reason::NotVerified),
        );
        assert!(retry_of(&l.plan, &l.lines[0], true).is_some());
        // A line for an app the plan does not have is not retried.
        let stray = line(Kind::App, "Unknown", ItemStatus::Failed, None);
        assert!(retry_of(&l.plan, &stray, true).is_none());
        assert!(
            retry_of(
                &l.plan,
                &line(Kind::App, "Writer", ItemStatus::SkippedByUser, None),
                true
            )
            .is_none()
        );
    }

    /// The retry plan holds exactly the picked items: other apps held, other settings left out,
    /// no app data.
    #[test]
    fn the_retry_plan_holds_only_the_picked_items() {
        let l = last();
        let picked = [&l.lines[0], &l.lines[4]];
        let mut plan = l.plan.clone();
        plan.special = vec![pctwin_fullmove::extras::SpecialLine {
            name: "Mail".into(),
            step: pctwin_fullmove::extras::SpecialStep::Copy,
            files: 2,
            secret: false,
            ids: vec![],
            sites: false,
        }];
        plan.special_not_checked = 2;
        plan.extras_refused = true;
        let part = part_plan(&plan, &picked);
        assert_eq!(part.held, [1, 2, 3]);
        let install: Vec<&str> = part
            .to_install()
            .iter()
            .map(|e| e.app.name.as_str())
            .collect();
        assert_eq!(install, ["Writer"]);
        assert_eq!(part.groups.len(), 1);
        assert_eq!(part.groups[0].label, "Writer settings");
        assert!(part.special.is_empty());
        assert_eq!(part.special_not_checked, 0);
        assert!(!part.extras_refused);
        // Already held stays held.
        let mut plan = l.plan.clone();
        plan.held = vec![0];
        assert_eq!(part_plan(&plan, &picked).held, [0, 1, 2, 3]);
    }

    /// A retry's new result replaces only the line it retried; every other line, finished or
    /// not, is kept exactly as it was.
    #[test]
    fn successful_results_are_kept_and_only_retried_lines_change() {
        let mut lines = last().lines;
        let before = lines.clone();
        let new = vec![
            line(Kind::App, "Writer", ItemStatus::Reinstalled, None),
            // The retry's other lines (held apps) are not picked: ignored.
            line(
                Kind::App,
                "Viewer",
                ItemStatus::NeedsAction,
                Some(Reason::FinishByHand),
            ),
            line(
                Kind::Settings,
                "Writer settings",
                ItemStatus::Failed,
                Some(Reason::NotVerified),
            ),
        ];
        let picked = vec![
            (Kind::App, Title::Name("Writer".into())),
            (Kind::Settings, Title::Name("Writer settings".into())),
        ];
        let fixed = merge(&mut lines, &new, &picked);
        assert_eq!(fixed, 1);
        assert_eq!(lines[0].status, ItemStatus::Reinstalled);
        assert_eq!(lines[1], before[1], "a finished result is kept");
        assert_eq!(lines[4].status, ItemStatus::Failed);
        for i in [2, 3, 5, 6, 7, 8] {
            assert_eq!(lines[i], before[i]);
        }
        // A line already finished is never counted as fixed again.
        let fixed = merge(&mut lines, &new[..1], &picked);
        assert_eq!(fixed, 0);
    }
}
