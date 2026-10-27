//! From what the old laptop held to what the new laptop gets (Phase 2.4 plan sections 4 to 6).
//!
//! [`plan`] takes the old laptop's [`Capture`], the new laptop's [`Target`] probe and the
//! person's [`Choices`], and gives each setting one declared [`Outcome`] (`exact`, `equivalent`,
//! `guided`, `unsupported` or `not-applicable`) and one [`Action`]. Only [`Action::Apply`]
//! changes anything, and only through the executor (its recipe: [`Planned::recipe`]); every
//! other action is a handoff, a guided step, a request to the administrator helper, a wait for a
//! dependency or nothing at all.
//!
//! The rules, each tested:
//! - An equivalent that changes the meaning (an automatic theme on a system that has none, a
//!   browser the new system does not have) waits for the person's choice (T24-006).
//! - A setting the new laptop's organisation manages is left alone (T24-013).
//! - Privacy approvals and security controls are never settings: they are listed as "approve
//!   again" or "left as the new laptop has it", never copied (T24-014, T24-023).
//! - A desktop picture is applied only once its file has arrived inside the person's folders on
//!   the new laptop; a cloud placeholder waits (T24-007); a path outside them is refused (T24-024).
//! - Display positions are never copied; scaling is guided (T24-008).
//! - A printer that is not on the new laptop needs choosing again (T24-016).
//! - The new laptop's stronger accessibility choice is kept unless the person allows the change.
//! - The settings go to the person the move maps them to, never to an account that merely has
//!   the same name (T24-004).

use std::collections::{BTreeMap, BTreeSet};

use pctwin_recipe::{Arch, Os, Platform, Recipe, RecipeError, RecipeId, SettingValue, Step};
use serde::{Deserialize, Serialize};

use crate::codec;
use crate::schema::{
    Route, SCHEMA_VERSION, SchemaError, SettingId, Support, System, check_value, support,
};

/// What reading one setting on the old laptop found.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Reading {
    Value(SettingValue),
    /// The system has nothing set (its default applies).
    NotSet,
    /// The organisation manages it: not the person's choice, not moved.
    Managed,
    /// It could not be read, and why (plain words).
    Unreadable(String),
    /// This system has no such setting.
    NotApplicable,
    /// PCTwin does not read it on this system yet.
    NotBuilt,
}

/// A privacy approval or security control the old laptop had. Recorded so the person knows,
/// never copied: approvals are given again on the new laptop, and a switched-off protection is
/// never switched off again because it was on the old one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Protected {
    CameraAccess,
    MicrophoneAccess,
    LocationAccess,
    ScreenRecording,
    FilesAccess,
    /// A protection that was switched off on the old laptop.
    SecurityOff(SecurityControl),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SecurityControl {
    Firewall,
    SmartScreen,
    Gatekeeper,
    DownloadScan,
}

/// Everything read on the old laptop: one reading per setting on the list, and the protected
/// things noted. Nothing else is read (T24-013: "do not crawl").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capture {
    pub schema: u32,
    pub system: System,
    pub readings: BTreeMap<SettingId, Reading>,
    #[serde(default)]
    pub protected: BTreeSet<Protected>,
}

impl Capture {
    pub fn new(system: System) -> Self {
        Self {
            schema: SCHEMA_VERSION,
            system,
            readings: BTreeMap::new(),
            protected: BTreeSet::new(),
        }
    }

    /// How many settings on the list were read with a value, and how many there are.
    pub fn coverage(&self) -> (usize, usize) {
        let read = self
            .readings
            .values()
            .filter(|r| matches!(r, Reading::Value(_) | Reading::NotSet))
            .count();
        (read, SettingId::ALL.len())
    }

    /// Reads a saved capture: a newer list is refused, every value is checked again.
    pub fn from_json(bytes: &[u8]) -> Result<Self, PlanError> {
        let c: Capture =
            serde_json::from_slice(bytes).map_err(|e| PlanError::Damaged(e.to_string()))?;
        if c.schema == 0 || c.schema > SCHEMA_VERSION {
            return Err(PlanError::NewerSchema(c.schema));
        }
        for (id, r) in &c.readings {
            if let Reading::Value(v) = r {
                check_value(*id, v)?;
            }
        }
        Ok(c)
    }
}

/// Where a file the old laptop pointed at is on the new laptop, from the move's item map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Arrival {
    /// It arrived, here.
    Arrived(String),
    /// Not yet (a cloud placeholder to download first, or still being copied): a dependency.
    NotYet,
    /// It is not part of the move.
    NotMoved,
}

/// The move's item map: old path to where it is now.
pub trait PathMap {
    fn arrival(&self, old_path: &str) -> Arrival;
}

impl PathMap for BTreeMap<String, Arrival> {
    fn arrival(&self, old_path: &str) -> Arrival {
        self.get(old_path).cloned().unwrap_or(Arrival::NotMoved)
    }
}

/// What the new laptop has now, probed before planning.
#[derive(Debug, Clone, Default)]
pub struct Target {
    pub current: BTreeMap<SettingId, Reading>,
    /// Settings the organisation manages on the new laptop.
    pub managed: BTreeSet<SettingId>,
    /// The person's folders on the new laptop (full paths): a desktop picture must be inside one.
    pub roots: Vec<String>,
    /// Browsers installed on the new laptop (neutral names).
    pub browsers: BTreeSet<String>,
    /// Printers on the new laptop, by name.
    pub printers: BTreeSet<String>,
}

/// The person's answers to the questions planning asks.
#[derive(Debug, Clone, PartialEq)]
pub enum Choice {
    /// Use this value (one of the options offered).
    Use(SettingValue),
    /// Keep what the new laptop has.
    Keep,
    /// Allow the change (lowering the new laptop's accessibility setting; startup apps).
    Allow,
}

pub type Choices = BTreeMap<SettingId, Choice>;

/// Who the settings belong to, as the move mapped people (accounts by their stable IDs, never
/// by name).
#[derive(Debug, Clone)]
pub struct People<'a> {
    pub old_account: &'a str,
    pub new_account: &'a str,
    /// Every (old account, new account) pair the move approved.
    pub mapped: &'a [(String, String)],
}

/// How a setting's meaning carries over.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    /// The same value with the same meaning.
    Exact,
    /// A close value on the new system; the person chooses when the meaning changes.
    Equivalent,
    /// The person does it on the system's own page.
    Guided,
    /// PCTwin cannot move it (managed, unreadable, protected).
    Unsupported,
    /// The new system has no such setting.
    NotApplicable,
}

/// Something the setting waits for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dependency {
    /// A file (the desktop picture) that has not arrived yet.
    File(String),
    /// An app (a browser) that 2.1 installs first.
    App(String),
}

/// What happens to a setting.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Set it, through the executor's journaled compare-and-write.
    Apply(SettingValue),
    /// The new laptop already holds exactly this.
    AlreadySet,
    /// The person picks first; nothing changes until then.
    NeedsChoice {
        options: Vec<SettingValue>,
        why: &'static str,
    },
    /// Open the system's own page for the person's choice, then read the result back.
    Handoff(SettingValue),
    /// The person does it; the old value is shown.
    Guided(Option<SettingValue>),
    /// Ask the administrator helper's fixed action.
    Privileged(SettingValue),
    /// Shown as its own choice, unticked.
    OptIn(SettingValue),
    /// Waits for a file or an app.
    Wait(Dependency),
    /// Left as it is, and why.
    Leave(&'static str),
}

/// One setting's plan.
#[derive(Debug, Clone, PartialEq)]
pub struct Planned {
    pub id: SettingId,
    pub outcome: Outcome,
    pub action: Action,
    /// The old laptop's value, if read.
    pub old: Option<SettingValue>,
    /// The new laptop's value before the move, if read.
    pub before: Option<SettingValue>,
}

/// The whole settings plan.
#[derive(Debug, Clone, PartialEq)]
pub struct SettingsPlan {
    pub system: System,
    pub settings: Vec<Planned>,
    /// Privacy approvals and switched-off protections: listed, never copied.
    pub protected: Vec<(Protected, &'static str)>,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum PlanError {
    #[error(transparent)]
    Schema(#[from] SchemaError),
    #[error("these settings belong to another person on the new laptop")]
    OwnerMismatch,
    #[error("these settings were read by a newer PCTwin (list {0}); update PCTwin")]
    NewerSchema(u32),
    #[error("the saved settings are damaged: {0}")]
    Damaged(String),
    #[error("a settings recipe could not be made: {0}")]
    Recipe(String),
}

impl From<RecipeError> for PlanError {
    fn from(e: RecipeError) -> Self {
        PlanError::Recipe(e.to_string())
    }
}

pub const APPROVE_AGAIN: &str = "you give this permission again on the new laptop when an app asks";
pub const PROTECTION_KEPT: &str =
    "this protection was off on the old laptop; it stays on, as the new laptop has it";
pub const MANAGED: &str = "your organisation manages this setting on the new laptop";
pub const NOT_READ: &str = "PCTwin could not read this on the old laptop";
pub const OUTSIDE: &str = "the picture is not inside your folders on the new laptop";
pub const NOT_MOVED: &str = "the picture was not part of the move";
pub const KEPT_STRONGER: &str =
    "the new laptop has a stronger accessibility setting; it is kept unless you choose otherwise";
pub const NO_AUTO: &str = "the new laptop has no automatic light and dark; choose one";
pub const NO_BROWSER: &str = "this browser does not exist on the new laptop's system; choose one";
pub const NO_PRINTER: &str = "this printer is not on the new laptop; choose a printer again";
pub const NO_LAYOUT: &str = "the new laptop names this keyboard differently; add it in Settings";
pub const POSITIONS: &str =
    "screen positions are not copied: the new laptop's screens are arranged as they are";
pub const KEPT_BY_YOU: &str = "you chose to keep the new laptop's setting";

/// Plans every setting on the list.
pub fn plan(
    capture: &Capture,
    target_system: System,
    target: &Target,
    files: &dyn PathMap,
    choices: &Choices,
    people: &People<'_>,
) -> Result<SettingsPlan, PlanError> {
    let pair = (
        people.old_account.to_string(),
        people.new_account.to_string(),
    );
    if !people.mapped.contains(&pair) {
        return Err(PlanError::OwnerMismatch);
    }
    if capture.schema == 0 || capture.schema > SCHEMA_VERSION {
        return Err(PlanError::NewerSchema(capture.schema));
    }
    let mut settings = Vec::new();
    for id in SettingId::ALL {
        let reading = capture
            .readings
            .get(&id)
            .cloned()
            .unwrap_or(Reading::NotBuilt);
        if let Reading::Value(v) = &reading {
            check_value(id, v)?;
        }
        let before = match target.current.get(&id) {
            Some(Reading::Value(v)) => Some(v.clone()),
            _ => None,
        };
        let p = plan_one(
            id,
            capture.system,
            &reading,
            target_system,
            target,
            files,
            choices.get(&id),
            before,
        );
        if let Action::Apply(v) = &p.action {
            check_value(id, v)?;
        }
        settings.push(p);
    }
    let protected = capture
        .protected
        .iter()
        .map(|p| {
            let why = match p {
                Protected::SecurityOff(_) => PROTECTION_KEPT,
                _ => APPROVE_AGAIN,
            };
            (*p, why)
        })
        .collect();
    Ok(SettingsPlan {
        system: target_system,
        settings,
        protected,
    })
}

#[allow(clippy::too_many_arguments)]
fn plan_one(
    id: SettingId,
    from: System,
    reading: &Reading,
    to: System,
    target: &Target,
    files: &dyn PathMap,
    choice: Option<&Choice>,
    before: Option<SettingValue>,
) -> Planned {
    let old = match reading {
        Reading::Value(v) => Some(v.clone()),
        _ => None,
    };
    let done = |outcome, action| Planned {
        id,
        outcome,
        action,
        old: old.clone(),
        before: before.clone(),
    };
    let sup = support(to, id);
    if sup == Support::NotApplicable {
        return done(
            Outcome::NotApplicable,
            Action::Leave("the new laptop has no such setting"),
        );
    }
    let Some(value) = old.clone() else {
        let why = match reading {
            Reading::Managed => "the old laptop's organisation managed this setting",
            Reading::NotSet | Reading::NotApplicable => "the old laptop used the system's default",
            _ => NOT_READ,
        };
        return done(Outcome::Unsupported, Action::Leave(why));
    };
    if target.managed.contains(&id) {
        return done(Outcome::Unsupported, Action::Leave(MANAGED));
    }
    if choice == Some(&Choice::Keep) {
        return done(Outcome::Exact, Action::Leave(KEPT_BY_YOU));
    }
    // Map the value onto the new system: (outcome, value to use) or an action that stops here.
    let mapped: Result<(Outcome, SettingValue), Action> = match id.route() {
        Route::Handoff => {
            return match id {
                SettingId::DefaultBrowser => browser(&value, to, target, choice, done),
                _ => done(Outcome::Guided, Action::Handoff(value)),
            };
        }
        Route::Guided => {
            return match id {
                SettingId::DefaultPrinter => match &value {
                    SettingValue::Text(name) if target.printers.contains(name) => {
                        done(Outcome::Guided, Action::Guided(Some(value)))
                    }
                    _ => done(
                        Outcome::Guided,
                        Action::NeedsChoice {
                            options: target
                                .printers
                                .iter()
                                .map(|p| SettingValue::Text(p.clone()))
                                .collect(),
                            why: NO_PRINTER,
                        },
                    ),
                },
                SettingId::DisplayScaling => done(Outcome::Guided, Action::Guided(Some(value))),
                _ => done(Outcome::Guided, Action::Guided(Some(value))),
            };
        }
        Route::Privileged => {
            return done(
                if from == to {
                    Outcome::Exact
                } else {
                    Outcome::Equivalent
                },
                Action::Privileged(value),
            );
        }
        Route::OptIn => {
            return match choice {
                Some(Choice::Allow) => done(Outcome::Guided, Action::Guided(Some(value))),
                _ => done(Outcome::Guided, Action::OptIn(value)),
            };
        }
        Route::Native => map_native(id, from, to, &value, target, files),
    };
    let (outcome, mut new_value) = match mapped {
        Ok(m) => m,
        Err(action) => {
            let outcome = match action {
                Action::Wait(_) => Outcome::Exact,
                Action::NeedsChoice { .. } => Outcome::Equivalent,
                Action::Guided(_) => Outcome::Guided,
                _ => Outcome::Unsupported,
            };
            // The person's pick for a question.
            if let (Action::NeedsChoice { options, .. }, Some(Choice::Use(v))) = (&action, choice)
                && options.contains(v)
            {
                return finish(
                    id,
                    Outcome::Equivalent,
                    v.clone(),
                    to,
                    done,
                    &before,
                    choice,
                );
            }
            return done(outcome, action);
        }
    };
    if sup == Support::Guided {
        return done(Outcome::Guided, Action::Guided(Some(new_value)));
    }
    if let Some(Choice::Use(v)) = choice
        && outcome == Outcome::Equivalent
    {
        new_value = v.clone();
    }
    finish(id, outcome, new_value, to, done, &before, choice)
}

/// The last checks before a native change: already set, and the new laptop's stronger
/// accessibility choice.
fn finish(
    id: SettingId,
    outcome: Outcome,
    value: SettingValue,
    to: System,
    done: impl Fn(Outcome, Action) -> Planned,
    before: &Option<SettingValue>,
    choice: Option<&Choice>,
) -> Planned {
    if support(to, id) != Support::Native {
        return done(Outcome::Guided, Action::Guided(Some(value)));
    }
    if before.as_ref() == Some(&value) {
        return done(outcome, Action::AlreadySet);
    }
    if id.accessibility() && lowers(before.as_ref(), &value) && choice != Some(&Choice::Allow) {
        return done(
            outcome,
            Action::NeedsChoice {
                options: vec![value],
                why: KEPT_STRONGER,
            },
        );
    }
    done(outcome, Action::Apply(value))
}

/// Whether `new` is a weaker accessibility setting than `before`.
fn lowers(before: Option<&SettingValue>, new: &SettingValue) -> bool {
    match (before, new) {
        (Some(SettingValue::Int(b)), SettingValue::Int(n)) => n < b,
        (Some(SettingValue::Bool(true)), SettingValue::Bool(false)) => true,
        _ => false,
    }
}

fn map_native(
    id: SettingId,
    from: System,
    to: System,
    value: &SettingValue,
    target: &Target,
    files: &dyn PathMap,
) -> Result<(Outcome, SettingValue), Action> {
    let same = from == to;
    let exact = |v: SettingValue| Ok((Outcome::Exact, v));
    match (id, value) {
        (SettingId::Theme, SettingValue::Text(t)) if t == "auto" && to != System::Mac => {
            Err(Action::NeedsChoice {
                options: vec![
                    SettingValue::Text("light".into()),
                    SettingValue::Text("dark".into()),
                ],
                why: NO_AUTO,
            })
        }
        (SettingId::PointerSpeed, SettingValue::Int(p)) => {
            let q = codec::quantize_speed(to, *p).ok_or(Action::Guided(Some(value.clone())))?;
            let outcome = if same && q == *p {
                Outcome::Exact
            } else {
                Outcome::Equivalent
            };
            Ok((outcome, SettingValue::Int(q)))
        }
        (SettingId::Wallpaper, SettingValue::Text(old_path)) => match files.arrival(old_path) {
            Arrival::Arrived(new_path) => {
                if inside(&new_path, &target.roots) {
                    exact(SettingValue::Text(new_path))
                } else {
                    Err(Action::Leave(OUTSIDE))
                }
            }
            Arrival::NotYet => Err(Action::Wait(Dependency::File(old_path.clone()))),
            Arrival::NotMoved => Err(Action::Leave(NOT_MOVED)),
        },
        (SettingId::KeyboardLayouts, SettingValue::Text(list)) => {
            // Layouts are only ever added, in the old laptop's order after the new laptop's own,
            // so no layout the person types with now is taken away.
            let mut out: Vec<String> = match target.current.get(&id) {
                Some(Reading::Value(SettingValue::Text(t))) if !t.is_empty() => {
                    t.split(',').map(str::to_string).collect()
                }
                _ => Vec::new(),
            };
            for l in list.split(',').filter(|l| !l.is_empty()) {
                if codec::layout_from_neutral(to, l).is_none() {
                    return Err(Action::Guided(Some(value.clone())));
                }
                if !out.iter().any(|o| o == l) {
                    out.push(l.to_string());
                }
            }
            exact(SettingValue::Text(out.join(",")))
        }
        _ => exact(value.clone()),
    }
}

fn browser(
    value: &SettingValue,
    to: System,
    target: &Target,
    choice: Option<&Choice>,
    done: impl Fn(Outcome, Action) -> Planned,
) -> Planned {
    let SettingValue::Text(name) = value else {
        return done(Outcome::Unsupported, Action::Leave(NOT_READ));
    };
    if !codec::browser_exists_on(to, name) {
        let options: Vec<SettingValue> = target
            .browsers
            .iter()
            .map(|b| SettingValue::Text(b.clone()))
            .collect();
        if let Some(Choice::Use(v)) = choice
            && options.contains(v)
        {
            return done(Outcome::Equivalent, Action::Handoff(v.clone()));
        }
        return done(
            Outcome::Equivalent,
            Action::NeedsChoice {
                options,
                why: NO_BROWSER,
            },
        );
    }
    if !target.browsers.contains(name) {
        return done(Outcome::Exact, Action::Wait(Dependency::App(name.clone())));
    }
    done(Outcome::Exact, Action::Handoff(value.clone()))
}

/// Whether `path` is inside one of `roots` (whole folder names only, either separator, letters'
/// case ignored on Windows paths).
pub fn inside(path: &str, roots: &[String]) -> bool {
    if !crate::schema::full_path(path) {
        return false;
    }
    let norm = |s: &str| {
        let s = s.replace('\\', "/");
        if s.as_bytes().get(1) == Some(&b':') {
            s.to_lowercase()
        } else {
            s
        }
    };
    let p = norm(path);
    roots.iter().any(|r| {
        let r = norm(r.trim_end_matches(['/', '\\']));
        !r.is_empty() && p.len() > r.len() && p.starts_with(&r) && p.as_bytes()[r.len()] == b'/'
    })
}

impl Planned {
    /// The executor recipe that applies this setting: one checked `apply-setting` step, for the
    /// new laptop's system. `None` unless the action is [`Action::Apply`].
    pub fn recipe(&self, os: Os) -> Result<Option<Recipe>, PlanError> {
        let Action::Apply(value) = &self.action else {
            return Ok(None);
        };
        check_value(self.id, value)?;
        let id = RecipeId::new(&format!("pctwin.settings.{}", self.id.key()))?;
        let platforms = [Arch::X86_64, Arch::Aarch64]
            .into_iter()
            .map(|arch| Platform { os, arch })
            .collect();
        Ok(Some(Recipe::new(
            id,
            SCHEMA_VERSION,
            platforms,
            vec![Step::ApplySetting {
                key: self.id.key().to_string(),
                value: value.clone(),
            }],
        )?))
    }
}

/// Which system a recipe platform's OS is, for the systems that are one OS.
pub fn os_of(system: System) -> Os {
    match system {
        System::Windows => Os::Windows,
        System::Mac => Os::Macos,
        System::Gnome | System::Kde | System::OtherLinux => Os::Linux,
    }
}
