//! Changes during the move (Task List 2.9): when an app updates, a destination folder changes or
//! a project is edited after the person approved, only the affected steps are held, checked
//! again, and asked about again; everything else carries on.
//!
//! The rules are `pctwin_fullmove::drift`. Before the first change of the rebuild, every app of
//! the plan is looked up again on this laptop: one that was installed, updated or removed since
//! the plan was made is held (never installed over what is there now, never said to be here) and
//! ends needs-action; the screens are told ([`EVENT`]) and asked again ([`move_reapprove`]):
//!
//! - yes: each held app is looked at once more. One that is here now is left as it is (the
//!   person has it); one that is not is installed through the same rebuild as the move, if PCTwin
//!   installs it at all;
//! - no: each held app ends skipped-by-user.
//!
//! A project edited on the old laptop after approval is refused by the capture's own check
//! (`ChangedSinceApproval`): nothing of that app's data is placed here, and the rebuild adds its
//! line (`drift::data_changed`, `project-edited`, held step `app-data`). Yes leaves it for "Fix
//! these", which waits for the old laptop; no ends it skipped-by-user.
//!
//! A destination folder moved or replaced after approval ends the move before anything is
//! written: the file transfer checks that each approved folder's path still leads to the very
//! folder it holds (`pctwin_gate::Destinations::changed_since_approved`) just before the landing
//! rules go to the old laptop, and stops with `Stop::PlaceChanged`; the screens say so
//! (`move-place-changed-*`). On Windows the held folder and the folders above it cannot be moved
//! at all while PCTwin holds them; the check matters on a Mac and on Linux.

use pctwin_fullmove::drift::{Drift, Step, Why};
use pctwin_fullmove::plan::{Kind, Title};
use pctwin_types::ItemResult;
use serde::Serialize;
use tauri::{AppHandle, Runtime};

use super::repair::{Kept, RepairDone, RepairProblem, Store, merge, offer, run_part, with_kept};
use crate::guard_views::{bounded, name};

pctwin_types::listed_enum! {
    /// What changed after the approval.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum AfterWhy {
        /// An app on this laptop was updated, or installed, since the approval.
        AppUpdated,
        /// An app on this laptop was removed since the approval.
        AppRemoved,
        /// A folder things go to was moved, removed or replaced.
        DestinationChanged,
        /// A project or file was edited on the old laptop.
        ProjectEdited,
    }
}

pctwin_types::listed_enum! {
    /// The steps held for it, to be checked again.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum Recheck {
        Install,
        Settings,
        AppData,
        Files,
    }
}

/// One thing that changed after approval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangedAfter {
    /// The app, folder or file, shown exactly as it is.
    pub name: String,
    pub why: AfterWhy,
    pub steps: Vec<Recheck>,
}

/// What changed after approval: only these are asked about again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangesView {
    /// The fresh approval's version: an answer for another version is refused.
    pub generation: u64,
    pub lines: Vec<ChangedAfter>,
    pub more: u64,
    /// Steps held until the person answers.
    pub held: u64,
}

/// The event sent when something changed after approval.
pub const EVENT: &str = "move-changed";

fn why(w: Why) -> AfterWhy {
    match w {
        Why::AppUpdated => AfterWhy::AppUpdated,
        Why::AppRemoved => AfterWhy::AppRemoved,
        Why::DestinationChanged => AfterWhy::DestinationChanged,
        Why::ProjectEdited => AfterWhy::ProjectEdited,
    }
}

fn step(s: Step) -> Recheck {
    match s {
        Step::Install => Recheck::Install,
        Step::Settings => Recheck::Settings,
        Step::AppData => Recheck::AppData,
        Step::Files => Recheck::Files,
    }
}

/// What changed after approval in `k` and waits for the person (`None`: nothing).
pub fn view(k: &Kept) -> Option<ChangesView> {
    let drift = &k.last.drift;
    if drift.is_empty() {
        return None;
    }
    let held = drift.iter().map(|d| d.steps.len() as u64).sum();
    let (lines, more) = bounded(
        drift
            .iter()
            .map(|d| ChangedAfter {
                name: name(&d.name),
                why: why(d.why),
                steps: d.steps.iter().copied().map(step).collect(),
            })
            .collect(),
    );
    Some(ChangesView {
        generation: k.generation,
        lines,
        more,
        held,
    })
}

/// What changed after approval and waits for the person (`None`: nothing).
#[tauri::command]
pub fn move_changes() -> Option<ChangesView> {
    with_kept(|k| k.as_ref().and_then(view))
}

/// The held apps of `drift` (their places in the plan's app list).
fn held_apps(drift: &[Drift]) -> Vec<u32> {
    drift.iter().filter_map(|d| d.app).collect()
}

/// The person said no: each held app, and the app data of each project edited after approval,
/// ends skipped-by-user; nothing waits any more.
pub fn decline(k: &mut Kept) {
    let names: Vec<Title> = held_apps(&k.last.drift)
        .iter()
        .filter_map(|at| k.last.plan.apps.get(*at as usize))
        .map(|e| Title::Name(name(&e.app.name)))
        .collect();
    let data: Vec<Title> = k
        .last
        .drift
        .iter()
        .filter(|d| d.why == Why::ProjectEdited)
        .map(|d| Title::Name(name(&d.name)))
        .collect();
    let skipped = ItemResult::skipped_by_user();
    for l in &mut k.last.lines {
        let held = match l.kind {
            Kind::App => names.contains(&l.title),
            Kind::Special => data.contains(&l.title),
            Kind::Settings => false,
        };
        if held {
            l.status = skipped.status();
            l.reason = skipped.reason();
        }
    }
    k.last.drift.clear();
    k.bump();
}

/// The person said yes: the plan as it is now. With this laptop's apps `installed_now`, each held
/// app that is here now is left as it is (its line goes: it is the person's, not a result of the
/// move); each that is not is released to be installed. The places of the released apps.
pub fn approve_now(k: &mut Kept, installed_now: &pctwin_record::AppList) -> Vec<u32> {
    use pctwin_apps::routes::Here;
    let mut release = Vec::new();
    let mut here_now = Vec::new();
    for at in held_apps(&k.last.drift) {
        let Some(e) = k.last.plan.apps.get(at as usize) else {
            continue;
        };
        // For an app from another kind of laptop, its edition here is what counts.
        match pctwin_fullmove::plan::here_of(e.subject(), installed_now) {
            Here::Present { .. } => here_now.push(Title::Name(name(&e.app.name))),
            _ => release.push(at),
        }
    }
    k.last
        .lines
        .retain(|l| !(l.kind == Kind::App && here_now.contains(&l.title)));
    k.last.plan.held.retain(|at| !release.contains(at));
    k.last.drift.clear();
    k.bump();
    release
}

/// The person's fresh answer for the held steps of `generation`: go ahead with them as they are
/// now (`approve`), or leave them out. What was retried, as "Fix these" says it.
#[tauri::command]
pub async fn move_reapprove<R: Runtime>(
    app: AppHandle<R>,
    generation: u64,
    approve: bool,
) -> Result<RepairDone, RepairProblem> {
    tauri::async_runtime::spawn_blocking(move || {
        reapprove(
            super::repair::store(),
            super::safe_stop::gate(),
            generation,
            approve,
            crate::fullmove::installs_here(),
            crate::fullmove::installed_here,
            |part, me, files| run_part(&app, part, me, files),
        )
    })
    .await
    .map_err(|_| RepairProblem::Failed)?
}

/// The fresh answer `approve` for the held steps of `generation` in `store`, with `installs`,
/// this laptop's apps now (`installed_now`, for the signed-in person) and `part` to install what
/// is released.
pub fn reapprove(
    store: &Store,
    gate: &'static super::safe_stop::SafeStop,
    generation: u64,
    approve: bool,
    installs: bool,
    installed_now: impl FnOnce(&str) -> pctwin_record::AppList,
    part: impl FnOnce(
        &pctwin_fullmove::plan::FullPlan,
        &str,
        Vec<(String, String)>,
    ) -> Result<(Vec<crate::fullmove::RebuiltLine>, Option<String>), RepairProblem>,
) -> Result<RepairDone, RepairProblem> {
    // Held before anything is looked at: no undo, move or retry can start until it ends.
    let _answering = gate
        .try_hold(super::safe_stop::BusyWith::Rebuilding)
        .ok_or(RepairProblem::Busy)?;
    let me = store.with(|k| {
        let k = k.as_ref().ok_or(RepairProblem::NothingToRepair)?;
        if k.generation != generation {
            return Err(RepairProblem::Stale);
        }
        if k.last.drift.is_empty() {
            return Err(RepairProblem::NothingToRepair);
        }
        Ok(k.last.me.clone())
    })?;
    let done = |k: &Kept, fixed: u64, undo_id: Option<String>| {
        let o = offer(k, installs);
        RepairDone {
            generation: o.generation,
            fixed,
            lines: o.lines,
            more: o.more,
            kept: o.kept,
            undo_id,
        }
    };
    // The results it answers for, still kept as they were (not spent, not changed meanwhile).
    let same = |k: &mut Option<Kept>, generation: u64| {
        k.as_mut()
            .filter(|k| k.generation == generation)
            .map(|_| ())
            .ok_or(RepairProblem::Stale)
    };
    if !approve {
        return store.with(|k| {
            same(k, generation)?;
            let k = k.as_mut().ok_or(RepairProblem::Failed)?;
            decline(k);
            Ok(done(k, 0, None))
        });
    }
    let installed_now = installed_now(&me);
    let (plan, files, wanted, approved) = store.with(|k| {
        same(k, generation)?;
        let k = k.as_mut().ok_or(RepairProblem::Failed)?;
        let release = approve_now(k, &installed_now);
        let titles: Vec<Title> = release
            .iter()
            .filter_map(|at| k.last.plan.apps.get(*at as usize))
            .map(|e| Title::Name(name(&e.app.name)))
            .collect();
        let picked: Vec<&crate::fullmove::RebuiltLine> = k
            .last
            .lines
            .iter()
            .filter(|l| l.kind == Kind::App && titles.contains(&l.title))
            .collect();
        let wanted: Vec<(Kind, Title)> = picked.iter().map(|l| (l.kind, l.title.clone())).collect();
        Ok::<_, RepairProblem>((
            super::repair::part_plan(&k.last.plan, &picked),
            k.last.files.clone(),
            wanted,
            k.generation,
        ))
    })?;
    if wanted.is_empty() {
        return store.with(|k| Ok(done(k.as_ref().ok_or(RepairProblem::Failed)?, 0, None)));
    }
    let (new, undo_id) = part(&plan, &me, files)?;
    store.with(|k| {
        // Merged only into the results this answer made: if they were spent or changed while it
        // ran, its own results stay in its own undo record and nothing is merged.
        same(k, approved)?;
        let k = k.as_mut().ok_or(RepairProblem::Failed)?;
        let fixed = merge(&mut k.last.lines, &new, &wanted);
        k.bump();
        Ok(done(k, fixed, undo_id))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deliver::repair::Last;
    use crate::fullmove::RebuiltLine;
    use pctwin_record::{AppList, AppMaker, AppSystem, FoundApp, FoundWhere, Owner, RegistryView};
    use pctwin_types::ItemStatus;
    use pctwin_types::Reason;

    fn line(n: &str, status: ItemStatus) -> RebuiltLine {
        RebuiltLine {
            title: Title::Name(n.into()),
            kind: Kind::App,
            count: 0,
            status,
            reason: Some(Reason::FinishByHand),
            notes: vec![],
            special_notes: vec![],
        }
    }

    /// A version the program never hands out in a test run.
    const G: u64 = 1 << 40;

    fn kept() -> Kept {
        let mut plan = crate::deliver::repair::tests::plan_with(&[
            ("Writer", true),
            ("Notes", true),
            ("Paint", true),
        ]);
        plan.held = vec![0, 1];
        Kept {
            generation: G,
            last: Last {
                plan,
                me: "1001".into(),
                files: vec![],
                lines: vec![
                    line("Writer", ItemStatus::NeedsAction),
                    line("Notes", ItemStatus::NeedsAction),
                    line("Paint", ItemStatus::Reinstalled),
                ],
                drift: vec![
                    Drift {
                        name: "Writer".into(),
                        why: Why::AppUpdated,
                        steps: vec![Step::Install],
                        app: Some(0),
                    },
                    Drift {
                        name: "Notes".into(),
                        why: Why::AppRemoved,
                        steps: vec![Step::Install],
                        app: Some(1),
                    },
                    Drift {
                        name: "Pictures".into(),
                        why: Why::DestinationChanged,
                        steps: vec![Step::Files, Step::AppData],
                        app: None,
                    },
                ],
            },
        }
    }

    #[test]
    fn what_changed_after_approval_is_shown_with_its_held_steps() {
        let v = view(&kept()).unwrap();
        assert_eq!(v.generation, G);
        assert_eq!(v.held, 4);
        let whys: Vec<AfterWhy> = v.lines.iter().map(|l| l.why).collect();
        assert_eq!(
            whys,
            [
                AfterWhy::AppUpdated,
                AfterWhy::AppRemoved,
                AfterWhy::DestinationChanged
            ]
        );
        assert_eq!(v.lines[2].steps, [Recheck::Files, Recheck::AppData]);
        let mut k = kept();
        k.last.drift.clear();
        assert_eq!(view(&k), None, "nothing waits: nothing is shown");
        assert_eq!(why(Why::ProjectEdited), AfterWhy::ProjectEdited);
        assert_eq!(step(Step::Settings), Recheck::Settings);
        assert_eq!(step(Step::Install), Recheck::Install);
    }

    /// Fresh-context review F5: app data edited on the old laptop after approval is shown, held
    /// (nothing of it was placed), and asked about: no ends it skipped-by-user; yes leaves it for
    /// "Fix these", which waits for the old laptop.
    #[test]
    fn app_data_edited_after_approval_is_asked_about_again() {
        let mut k = kept();
        k.last.drift = vec![pctwin_fullmove::drift::edited("Mail")];
        k.last.lines.push(RebuiltLine {
            kind: Kind::Special,
            ..line("Mail", ItemStatus::NeedsAction)
        });
        let v = view(&k).unwrap();
        assert_eq!(v.lines[0].why, AfterWhy::ProjectEdited);
        assert_eq!(v.lines[0].steps, [Recheck::AppData]);
        let mut no = k.clone();
        decline(&mut no);
        assert_eq!(no.last.lines[3].status, ItemStatus::SkippedByUser);
        assert_eq!(
            no.last.lines[0].status,
            ItemStatus::NeedsAction,
            "apps not held stay"
        );
        let mut yes = k.clone();
        assert!(approve_now(&mut yes, &empty()).is_empty());
        assert!(view(&yes).is_none());
        let o = offer(&yes, true);
        let mail = o
            .lines
            .iter()
            .find(|l| l.title == Title::Name("Mail".into()))
            .unwrap();
        assert_eq!(
            mail.waits_for,
            [crate::deliver::repair::Prerequisite::OldLaptop]
        );
    }

    /// No: the held apps end skipped-by-user; every other line is kept.
    #[test]
    fn a_no_skips_only_the_held_apps() {
        let mut k = kept();
        decline(&mut k);
        assert_eq!(k.last.lines[0].status, ItemStatus::SkippedByUser);
        assert_eq!(k.last.lines[0].reason, None);
        assert_eq!(k.last.lines[1].status, ItemStatus::SkippedByUser);
        assert_eq!(k.last.lines[2].status, ItemStatus::Reinstalled);
        assert!(k.last.drift.is_empty());
        assert_ne!(k.generation, G, "a new version");
    }

    /// Yes: the plan as it is now. An app here now is the person's and left as it is; one that is
    /// not here is released to install; nothing else is touched.
    #[test]
    fn a_yes_installs_only_what_is_still_missing_now() {
        let mut k = kept();
        let writer_here = AppList {
            found: vec![FoundApp {
                name: "Writer".into(),
                version: None,
                publisher: None,
                owner: Owner::Shared,
                system: AppSystem::Windows,
                found: FoundWhere::WindowsEntry {
                    view: RegistryView::Machine,
                    key: "Writer".into(),
                    product_code: Some("{Writer}".into()),
                },
                arch: pctwin_record::AppArch::UNKNOWN,
                maker: AppMaker::Unsigned,
                web: None,
                update_feed: None,
                installers: vec![],
                notes: vec![],
            }],
            not_read: vec![],
        };
        let release = approve_now(&mut k, &writer_here);
        assert_eq!(release, [1]);
        let names: Vec<&Title> = k.last.lines.iter().map(|l| &l.title).collect();
        assert_eq!(
            names,
            [&Title::Name("Notes".into()), &Title::Name("Paint".into())]
        );
        assert_eq!(k.last.plan.held, [0]);
        assert!(k.last.drift.is_empty());
        assert_ne!(k.generation, G, "a new version");
    }

    /// Review finding (30 October 2026): a held app from another kind of laptop is judged by its
    /// edition here, so it is never released to install over that edition already here.
    #[test]
    fn a_held_app_from_another_system_is_judged_by_its_edition_here() {
        let mut k = kept();
        let mut edition = k.last.plan.apps[1].app.clone();
        edition.system = AppSystem::Mac;
        edition.found = FoundWhere::MacBundle {
            bundle_id: "org.example.notes".into(),
            folder: "Notes.app".into(),
            app_store_id: None,
        };
        k.last.plan.apps[1].plan.stand_in = Some(edition.clone());
        let release = approve_now(
            &mut k,
            &AppList {
                found: vec![edition],
                not_read: vec![],
            },
        );
        assert_eq!(
            release,
            [0],
            "Notes' edition is here: only Writer is released"
        );
        assert_eq!(k.last.plan.held, [1]);
    }

    fn empty() -> AppList {
        AppList {
            found: vec![],
            not_read: vec![],
        }
    }

    /// Fresh-context review F1: a fresh approval is never answered from results an undo or a new
    /// move spent, and one whose results were spent while it installed merges nothing.
    #[test]
    fn a_fresh_approval_never_acts_on_spent_results() {
        use crate::deliver::safe_stop::{BusyWith, SafeStop};
        let gate: &'static SafeStop = Box::leak(Box::new(SafeStop::new()));
        let store = Store::new();
        store.with(|k| *k = Some(kept()));
        store.spend();
        let mut ran = false;
        for approve in [true, false] {
            let r = reapprove(
                &store,
                gate,
                G,
                approve,
                true,
                |_| empty(),
                |_, _, _| {
                    ran = true;
                    Ok((vec![], None))
                },
            );
            assert_eq!(r, Err(RepairProblem::NothingToRepair));
        }
        assert!(!ran);
        // Spent while it installed: nothing merged, nothing brought back.
        store.with(|k| *k = Some(kept()));
        let r = reapprove(
            &store,
            gate,
            G,
            true,
            true,
            |_| empty(),
            |part, _, _| {
                assert!(gate.try_hold(BusyWith::Undoing).is_none(), "held first");
                assert_eq!(part.to_install().len(), 2);
                store.spend();
                Ok((
                    vec![line("Writer", ItemStatus::Reinstalled)],
                    Some("u".into()),
                ))
            },
        );
        assert_eq!(r, Err(RepairProblem::Stale));
        assert!(store.with(|k| k.is_none()));
        // Replaced by newer results while it installed (re-check of 2.9): nothing merged into them.
        store.with(|k| *k = Some(kept()));
        let r = reapprove(
            &store,
            gate,
            G,
            true,
            true,
            |_| empty(),
            |_, _, _| {
                store.with(|k| {
                    *k = Some(Kept {
                        generation: G + 1,
                        ..kept()
                    })
                });
                Ok((
                    vec![line("Writer", ItemStatus::Reinstalled)],
                    Some("u".into()),
                ))
            },
        );
        assert_eq!(r, Err(RepairProblem::Stale));
        store.with(|k| {
            let k = k.as_ref().unwrap();
            assert_eq!(k.generation, G + 1);
            assert_eq!(
                k.last.lines[0].status,
                ItemStatus::NeedsAction,
                "left as it was"
            );
        });
        // Busy: refused before anything is looked at.
        store.with(|k| *k = Some(kept()));
        let undo = gate.hold(BusyWith::Undoing);
        let r = reapprove(
            &store,
            gate,
            G,
            false,
            true,
            |_| empty(),
            |_, _, _| Ok((vec![], None)),
        );
        assert_eq!(r, Err(RepairProblem::Busy));
        drop(undo);
        // Nothing changed meanwhile: the answer is merged.
        let done = reapprove(
            &store,
            gate,
            G,
            true,
            true,
            |_| empty(),
            |_, _, _| Ok((vec![line("Writer", ItemStatus::Reinstalled)], None)),
        )
        .unwrap();
        assert_eq!(done.fixed, 1);
        assert!(gate.view().safe);
    }
}
