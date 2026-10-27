//! The flow guard (Task List 2.8; Phase 2 plan 2.8 as adopted 26 October 2026): what the Phase 2
//! screens may see, and the only things they may ask for.
//!
//! The engine publishes a cut-down projection of one job: its id, its session number (epoch),
//! the plan generation the person is looking at, a version that grows with every update, the
//! display state, the screen's content ([`ScreenView`]), measured progress, and the intents
//! allowed now. The screens send back an [`Intent`] that names the job, epoch and plan
//! generation they showed. The guard checks every one again here, whatever the buttons looked
//! like:
//!
//! - no job, or the engine has not said yet: refused as "status unavailable", never as idle;
//! - another job, or an older session: refused (a late answer cannot move a newer job);
//! - an older plan generation: refused as "the plan changed, look again" (a stale approval
//!   cannot approve changed work);
//! - an intent not allowed in this state, or naming a line or drive the screen does not show:
//!   refused;
//! - the same intent twice for one generation (a double-click): taken once, answered "already
//!   taken" the second time, and handed to the engine once.
//!
//! There is no "approve everything", no "mark complete", no path, no URL and no command among the
//! intents. Nothing here decides anything: an accepted intent is queued for the engine part that
//! owns it ([`Guard::take`]), which checks its own authority again before acting. A screen
//! appearing, an animation ending or a timer running out sends nothing.

use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Runtime, State};

use crate::guard_views::ScreenView;

/// The event the screens listen to.
pub const EVENT: &str = "guard-view";

pctwin_types::listed_enum! {
    /// Every display state an operation can be in (plan 2.8 section 5). The screens show each in
    /// words, never by colour alone.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum DisplayState {
        NotStarted,
        Preparing,
        /// Waiting for the person to choose.
        AwaitingPerson,
        /// Waiting for the system's own prompt.
        AwaitingSystem,
        /// Waiting for an app's maker or an online service.
        AwaitingMaker,
        /// Copying, downloading or sending.
        Working,
        /// Checking what was done.
        Checking,
        /// Putting things in place.
        Applying,
        /// Asked to pause; it pauses at the next safe point, and is not paused yet.
        PauseRequested,
        /// Paused at a safe point.
        Paused,
        /// The old laptop or drive is not there.
        SourceUnavailable,
        /// Something must be fixed first.
        Blocked,
        /// Partly done: some parts did not finish.
        Partial,
        Retrying,
        /// Checking where things got to after a restart.
        Reconciling,
        Failed,
        /// Done, with exactly what was done in the report.
        Complete,
        /// Stopped; what was done is kept.
        Cancelled,
    }
}

pctwin_types::listed_enum! {
    /// What a screen may ask for. Each is checked against the job's state again. Every variant
    /// has fields (most none), so an unknown field is refused rather than ignored.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
    pub enum IntentKind {
        /// Approve exactly the plan generation shown.
        Approve {},
        /// Leave the items that changed out, instead of approving them again.
        LeaveOutChanged {},
        /// Keep a local copy of online-only files, or not.
        KeepLocalCopy { keep: bool },
        /// Carry on keeping copies on a connection that may charge for data.
        AllowMetered {},
        /// Approve the offload drive shown as a destination.
        ApproveOffload {},
        /// Choose the rescue drive at this place in the list shown.
        ChooseDrive { drive: u32 },
        /// Look again (a drive unlocked, a laptop connected).
        CheckAgain {},
        /// Open the system's own administrator prompt.
        AskSystem {},
        /// Carry on without what the prompt was for.
        NotNow {},
        /// Try one report line again.
        TryAgain { line: u32 },
        /// Look at what undo would do.
        PreviewUndo {},
        /// Undo, as previewed for this generation.
        Undo {},
        Pause {},
        Resume {},
        /// Go back to the step before.
        Back {},
    }
}

impl IntentKind {
    /// The intent's name without its details, as the allowed list holds it.
    pub fn name(&self) -> IntentName {
        match self {
            IntentKind::Approve {} => IntentName::Approve,
            IntentKind::LeaveOutChanged {} => IntentName::LeaveOutChanged,
            IntentKind::KeepLocalCopy { .. } => IntentName::KeepLocalCopy,
            IntentKind::AllowMetered {} => IntentName::AllowMetered,
            IntentKind::ApproveOffload {} => IntentName::ApproveOffload,
            IntentKind::ChooseDrive { .. } => IntentName::ChooseDrive,
            IntentKind::CheckAgain {} => IntentName::CheckAgain,
            IntentKind::AskSystem {} => IntentName::AskSystem,
            IntentKind::NotNow {} => IntentName::NotNow,
            IntentKind::TryAgain { .. } => IntentName::TryAgain,
            IntentKind::PreviewUndo {} => IntentName::PreviewUndo,
            IntentKind::Undo {} => IntentName::Undo,
            IntentKind::Pause {} => IntentName::Pause,
            IntentKind::Resume {} => IntentName::Resume,
            IntentKind::Back {} => IntentName::Back,
        }
    }
}

/// An intent's name, as the projection's allowed list carries it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum IntentName {
    Approve,
    LeaveOutChanged,
    KeepLocalCopy,
    AllowMetered,
    ApproveOffload,
    ChooseDrive,
    CheckAgain,
    AskSystem,
    NotNow,
    TryAgain,
    PreviewUndo,
    Undo,
    Pause,
    Resume,
    Back,
}

/// Work measured so far. No total: the screens show an indeterminate bar with what is known.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Measured {
    pub done: u64,
    pub total: Option<u64>,
    pub unit: Unit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Unit {
    Bytes,
    Items,
}

/// What the screens are shown of one job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Projection {
    pub job: String,
    pub epoch: u64,
    pub generation: u64,
    pub version: u64,
    pub display: DisplayState,
    pub allowed: Vec<IntentName>,
    pub progress: Option<Measured>,
    pub screen: ScreenView,
}

pctwin_types::listed_enum! {
    /// What the screens get when they ask.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    #[serde(tag = "kind", rename_all = "kebab-case")]
    pub enum Snapshot {
        /// The engine has not said: the screens show "status unavailable", never idle or done.
        Unavailable,
        Ready { view: Box<Projection> },
    }
}

/// An intent from the screens.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Intent {
    pub job: String,
    pub epoch: u64,
    pub generation: u64,
    pub intent: IntentKind,
}

pctwin_types::listed_enum! {
    /// Why an intent was refused, as a word key. Never the engine's own words: they could carry
    /// a name, a path or a secret.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(tag = "kind", rename_all = "kebab-case")]
    pub enum Refused {
        /// No job, or the engine has not said: status unavailable.
        Unavailable,
        /// It names another job.
        WrongJob,
        /// It belongs to an older session of this job.
        OldSession,
        /// The plan changed since the screen showed it: look again.
        PlanChanged,
        /// Not something this step can do now.
        NotNow,
        /// It names a line or drive the screen does not show, or a drive that is also a target.
        NotShown,
    }
}

pctwin_types::listed_enum! {
    /// What came of an intent the guard took.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(tag = "kind", rename_all = "kebab-case")]
    pub enum Answer {
        /// Taken, and handed to the engine.
        Taken,
        /// The same intent for this generation was already taken: nothing new is done.
        AlreadyTaken,
    }
}

/// An update the engine publishes for a job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Update {
    pub job: String,
    pub epoch: u64,
    pub generation: u64,
    pub display: DisplayState,
    pub allowed: Vec<IntentName>,
    pub progress: Option<Measured>,
    pub screen: ScreenView,
}

/// Why the engine's own update was not published.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stale {
    /// From another job than the one shown.
    WrongJob,
    /// From an older session of it.
    OldSession,
    /// From an older plan generation in the same session.
    OldGeneration,
}

/// The guard's record of the job shown.
#[derive(Debug, Default)]
pub struct Guard {
    shown: Option<Projection>,
    /// Intents taken for the generation shown.
    taken: Vec<IntentKind>,
    /// Intents taken and not yet collected by the engine.
    queue: Vec<Intent>,
    version: u64,
}

impl Guard {
    /// A new job, or a new session of one (after a restart or a new pairing): replaces whatever
    /// was shown, and starts with the engine checking where things got to. Nothing is allowed
    /// until the engine has looked.
    pub fn begin(&mut self, job: &str, epoch: u64) -> Projection {
        self.taken.clear();
        self.queue.clear();
        self.version += 1;
        let view = Projection {
            job: job.to_owned(),
            epoch,
            generation: 0,
            version: self.version,
            display: DisplayState::Reconciling,
            allowed: Vec::new(),
            progress: None,
            screen: ScreenView::Reconciling,
        };
        self.shown = Some(view.clone());
        view
    }

    /// The engine's update. One from another job, an older session or an older plan generation
    /// is refused, so a late result cannot change what a newer job shows. A new generation clears
    /// what was taken for the old one.
    pub fn publish(&mut self, update: Update) -> Result<Projection, Stale> {
        let Some(shown) = &self.shown else {
            return Err(Stale::WrongJob);
        };
        if update.job != shown.job {
            return Err(Stale::WrongJob);
        }
        if update.epoch != shown.epoch {
            return Err(Stale::OldSession);
        }
        if update.generation < shown.generation {
            return Err(Stale::OldGeneration);
        }
        if update.generation != shown.generation {
            self.taken.clear();
        }
        self.version += 1;
        let view = Projection {
            job: update.job,
            epoch: update.epoch,
            generation: update.generation,
            version: self.version,
            display: update.display,
            allowed: update.allowed,
            progress: update.progress,
            screen: update.screen,
        };
        self.shown = Some(view.clone());
        Ok(view)
    }

    /// The job ended: the screens show "status unavailable" until another begins.
    pub fn end(&mut self) {
        self.shown = None;
        self.taken.clear();
    }

    pub fn snapshot(&self) -> Snapshot {
        match &self.shown {
            Some(view) => Snapshot::Ready {
                view: Box::new(view.clone()),
            },
            None => Snapshot::Unavailable,
        }
    }

    /// Checks an intent from the screens, every rule again.
    pub fn intent(&mut self, intent: Intent) -> Result<Answer, Refused> {
        let Some(shown) = &self.shown else {
            return Err(Refused::Unavailable);
        };
        if intent.job != shown.job {
            return Err(Refused::WrongJob);
        }
        if intent.epoch != shown.epoch {
            return Err(Refused::OldSession);
        }
        if intent.generation != shown.generation {
            return Err(Refused::PlanChanged);
        }
        if !shown.allowed.contains(&intent.intent.name()) {
            return Err(Refused::NotNow);
        }
        match intent.intent {
            IntentKind::TryAgain { line } if !shown.screen.report_lines().contains(&line) => {
                return Err(Refused::NotShown);
            }
            IntentKind::ChooseDrive { drive } => {
                let at = drive as usize;
                if at >= shown.screen.rescue_drives() || shown.screen.rescue_drive_is_target(at) {
                    return Err(Refused::NotShown);
                }
            }
            _ => {}
        }
        if self.taken.contains(&intent.intent) {
            return Ok(Answer::AlreadyTaken);
        }
        self.taken.push(intent.intent);
        self.queue.push(intent);
        Ok(Answer::Taken)
    }

    /// The intents taken since the engine last looked, oldest first. Each is handed over once.
    pub fn take(&mut self) -> Vec<Intent> {
        std::mem::take(&mut self.queue)
    }
}

/// The guard, as the app holds it.
pub type AppGuard = Mutex<Guard>;

pub fn new_state() -> AppGuard {
    Mutex::new(Guard::default())
}

/// Publishes the engine's update and tells the screens. A stale update is dropped here, before
/// the screens could see it.
pub fn publish<R: Runtime>(
    app: &AppHandle<R>,
    guard: &AppGuard,
    update: Update,
) -> Result<(), Stale> {
    let view = guard
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .publish(update)?;
    let _ = app.emit(EVENT, &view);
    Ok(())
}

/// What the screens show for the job now. Asked for on opening (before any action is shown)
/// and after a missed update.
#[tauri::command]
pub fn guard_view(guard: State<'_, AppGuard>) -> Snapshot {
    guard.lock().unwrap_or_else(|e| e.into_inner()).snapshot()
}

/// An intent from a screen, checked again here.
#[tauri::command]
pub fn guard_intent(guard: State<'_, AppGuard>, intent: Intent) -> Result<Answer, Refused> {
    guard
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .intent(intent)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guard_views::{RescueDrive, RescueStage};

    fn update(generation: u64, allowed: Vec<IntentName>, screen: ScreenView) -> Update {
        Update {
            job: "job-1".into(),
            epoch: 4,
            generation,
            display: DisplayState::AwaitingPerson,
            allowed,
            progress: None,
            screen,
        }
    }

    fn changes() -> ScreenView {
        ScreenView::Changes {
            changes: vec![],
            more: 0,
            download_bytes: None,
            drives: vec![],
            permissions: vec![],
            restore_point: false,
        }
    }

    fn ready(generation: u64) -> Guard {
        let mut g = Guard::default();
        g.begin("job-1", 4);
        g.publish(update(
            generation,
            vec![IntentName::Approve, IntentName::Pause],
            changes(),
        ))
        .unwrap();
        g
    }

    fn ask(generation: u64, intent: IntentKind) -> Intent {
        Intent {
            job: "job-1".into(),
            epoch: 4,
            generation,
            intent,
        }
    }

    // T28-008: no authoritative state is "unavailable", never idle or done.
    #[test]
    fn with_no_job_the_status_is_unavailable_and_every_intent_refused() {
        let mut g = Guard::default();
        assert_eq!(g.snapshot(), Snapshot::Unavailable);
        assert_eq!(
            g.intent(ask(1, IntentKind::Approve {})),
            Err(Refused::Unavailable)
        );
        assert!(g.take().is_empty());
    }

    // T28-009: after a restart the job starts reconciling, with nothing allowed.
    #[test]
    fn a_new_session_shows_reconciling_and_allows_nothing_until_the_engine_looks() {
        let mut g = ready(2);
        let view = g.begin("job-1", 5);
        assert_eq!(view.display, DisplayState::Reconciling);
        assert_eq!(view.screen, ScreenView::Reconciling);
        assert!(view.allowed.is_empty());
        let mut old = ask(2, IntentKind::Approve {});
        old.epoch = 5;
        assert_eq!(g.intent(old), Err(Refused::PlanChanged));
    }

    // T28-002: a stale approval is refused by the engine, whatever the button showed.
    #[test]
    fn an_approval_of_an_older_plan_generation_is_refused() {
        let mut g = ready(2);
        g.publish(update(3, vec![IntentName::Approve], changes()))
            .unwrap();
        assert_eq!(
            g.intent(ask(2, IntentKind::Approve {})),
            Err(Refused::PlanChanged)
        );
        assert_eq!(g.intent(ask(3, IntentKind::Approve {})), Ok(Answer::Taken));
    }

    // T28-003 and T28-007: another job's or an older session's events and intents are refused.
    #[test]
    fn wrong_job_and_old_session_are_refused_both_ways() {
        let mut g = ready(2);
        let mut other = update(2, vec![], changes());
        other.job = "job-2".into();
        assert_eq!(g.publish(other), Err(Stale::WrongJob));
        let mut old = update(9, vec![], ScreenView::Reconciling);
        old.epoch = 3;
        assert_eq!(g.publish(old), Err(Stale::OldSession));
        assert_eq!(
            g.publish(update(1, vec![], changes())),
            Err(Stale::OldGeneration)
        );
        let mut wrong = ask(2, IntentKind::Approve {});
        wrong.job = "job-2".into();
        assert_eq!(g.intent(wrong), Err(Refused::WrongJob));
        let mut late = ask(2, IntentKind::Approve {});
        late.epoch = 3;
        assert_eq!(g.intent(late), Err(Refused::OldSession));
        // What is shown did not move.
        let Snapshot::Ready { view } = g.snapshot() else {
            panic!()
        };
        assert_eq!(view.generation, 2);
    }

    // T28-006: a double-click is one operation.
    #[test]
    fn the_same_intent_twice_is_handed_to_the_engine_once() {
        let mut g = ready(2);
        assert_eq!(g.intent(ask(2, IntentKind::Approve {})), Ok(Answer::Taken));
        assert_eq!(
            g.intent(ask(2, IntentKind::Approve {})),
            Ok(Answer::AlreadyTaken)
        );
        assert_eq!(g.take().len(), 1);
        assert!(g.take().is_empty());
    }

    #[test]
    fn an_intent_not_allowed_now_is_refused() {
        let mut g = ready(2);
        assert_eq!(g.intent(ask(2, IntentKind::Undo {})), Err(Refused::NotNow));
        assert_eq!(
            g.intent(ask(2, IntentKind::Resume {})),
            Err(Refused::NotNow)
        );
    }

    #[test]
    fn try_again_and_choose_drive_must_name_what_the_screen_shows() {
        let mut g = Guard::default();
        g.begin("job-1", 4);
        let drives = vec![
            RescueDrive {
                drive: "Old disk".into(),
                size: 1,
                file_system: "NTFS".into(),
                is_target: false,
            },
            RescueDrive {
                drive: "New laptop".into(),
                size: 1,
                file_system: "NTFS".into(),
                is_target: true,
            },
        ];
        let screen = ScreenView::Rescue {
            stage: RescueStage::PickDrive {
                drives,
                refused: None,
            },
        };
        g.publish(update(1, vec![IntentName::ChooseDrive], screen))
            .unwrap();
        assert_eq!(
            g.intent(ask(1, IntentKind::ChooseDrive { drive: 1 })),
            Err(Refused::NotShown)
        );
        assert_eq!(
            g.intent(ask(1, IntentKind::ChooseDrive { drive: 7 })),
            Err(Refused::NotShown)
        );
        assert_eq!(
            g.intent(ask(1, IntentKind::ChooseDrive { drive: 0 })),
            Ok(Answer::Taken)
        );
        let report = ScreenView::Report {
            lines: vec![],
            more: 0,
            safe_in: vec![],
        };
        g.publish(update(1, vec![IntentName::TryAgain], report))
            .unwrap();
        assert_eq!(
            g.intent(ask(1, IntentKind::TryAgain { line: 0 })),
            Err(Refused::NotShown)
        );
    }

    // T28-001, T28-027: the screens can say only these intents; nothing else parses.
    #[test]
    fn no_generic_or_shortcut_intent_exists() {
        for bad in [
            r#"{"kind":"approve-all"}"#,
            r#"{"kind":"mark-complete"}"#,
            r#"{"kind":"run","command":"cmd"}"#,
            r#"{"kind":"open","path":"C:\\"}"#,
            r#"{"kind":"windows-approved"}"#,
            r#"{"kind":"approve","all":true}"#,
        ] {
            assert!(
                serde_json::from_str::<IntentKind>(bad).is_err(),
                "{bad} must not parse"
            );
        }
        // What the screens send (guard.ts `IntentKind`) parses, each to its own intent.
        for good in [
            r#"{"kind":"approve"}"#,
            r#"{"kind":"keep-local-copy","keep":true}"#,
            r#"{"kind":"choose-drive","drive":0}"#,
            r#"{"kind":"try-again","line":2}"#,
            r#"{"kind":"leave-out-changed"}"#,
            r#"{"kind":"preview-undo"}"#,
        ] {
            assert!(
                serde_json::from_str::<IntentKind>(good).is_ok(),
                "{good} must parse"
            );
        }
        for name in IntentKind::VARIANTS {
            let lower = name.to_lowercase();
            for word in [
                "approveall",
                "complete",
                "command",
                "shell",
                "path",
                "url",
                "file",
            ] {
                assert!(!lower.contains(word), "{name}");
            }
        }
        let extra =
            r#"{"job":"j","epoch":1,"generation":1,"intent":{"kind":"approve"},"force":true}"#;
        assert!(serde_json::from_str::<Intent>(extra).is_err());
    }

    // T28-012: pause asked for is not paused.
    #[test]
    fn pause_requested_and_paused_are_different_states() {
        assert_ne!(DisplayState::PauseRequested, DisplayState::Paused);
        let tags: Vec<_> = [DisplayState::PauseRequested, DisplayState::Paused]
            .iter()
            .map(|s| {
                serde_json::to_value(s)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .collect();
        assert_eq!(tags, ["pause-requested", "paused"]);
    }

    #[test]
    fn a_refusal_carries_a_key_only() {
        let v = serde_json::to_value(Refused::PlanChanged).unwrap();
        assert_eq!(v, serde_json::json!({ "kind": "plan-changed" }));
    }
}
