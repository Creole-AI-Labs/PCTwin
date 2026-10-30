//! From what the old laptop held to what the new laptop gets (Phase 2.4 plan sections 4 to 6),
//! for whatever rows the catalogue holds.
//!
//! [`plan`] takes the old laptop's [`Capture`], the new laptop's [`Target`] probe and the
//! person's [`Choices`], and gives each catalogue row one declared [`Outcome`] (`exact`,
//! `equivalent`, `guided`, `unsupported` or `not-applicable`) and one [`Action`]. Only
//! [`Action::Apply`] changes anything, and only through the executor ([`Planned::recipe`]).
//!
//! The rules, each tested:
//! - What the new system cannot hold exactly is an equivalent: the nearest value it can hold
//!   (pointer speed), or, when the meaning changes (an automatic theme on a system without one),
//!   a question with the values it can hold (T24-006).
//! - A setting the new laptop's organisation manages is left alone (T24-013).
//! - Privacy approvals and switched-off protections are listed, never copied (T24-014, T24-023).
//! - A file setting is applied only once its file has arrived inside the person's folders on the
//!   new laptop; a cloud placeholder waits (T24-007); a path outside them is refused (T24-024).
//! - A device that is not on the new laptop is chosen again (T24-016).
//! - The new laptop's stronger accessibility choice is kept unless the person allows the change.
//! - Lists marked add-only only ever grow (keyboard layouts: no layout in use disappears).
//! - Settings go to the person the move maps them to, never to a same-named account (T24-004).

use std::collections::{BTreeMap, BTreeSet};

use pctwin_recipe::{Arch, Os, Platform, Recipe, RecipeError, RecipeId, SettingValue, Step};
use serde::{Deserialize, Serialize};

use crate::catalogue::{Catalogue, Merge, Place, Protected, Route, Row, SettingId, System};
use crate::codec;
use crate::value::{ValueError, ValueType, inside};

/// The capture format.
pub const CAPTURE_FORMAT: u32 = 1;

/// What reading one setting on the old laptop found.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Reading {
    Value(SettingValue),
    /// Nothing set: the system's default applies.
    NotSet,
    /// The organisation manages it: not the person's choice, not moved.
    Managed,
    /// It could not be read, and why (plain words).
    Unreadable(String),
    /// The system has no such setting.
    NotApplicable,
    /// The catalogue has no place for it on this system yet.
    NotBuilt,
}

/// A setting found in a known store that the catalogue has no row for yet: counted and named
/// (never its value), so the catalogue can grow.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Found {
    pub store: String,
    pub name: String,
}

/// Everything read on the old laptop.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capture {
    pub format: u32,
    /// The catalogue revision it was read with.
    pub catalogue: u64,
    pub system: System,
    pub readings: BTreeMap<SettingId, Reading>,
    #[serde(default)]
    pub protected: BTreeSet<Protected>,
    /// Found, not yet supported.
    #[serde(default)]
    pub unsupported: Vec<Found>,
}

impl Capture {
    pub fn new(system: System, catalogue: u64) -> Self {
        Self {
            format: CAPTURE_FORMAT,
            catalogue,
            system,
            readings: BTreeMap::new(),
            protected: BTreeSet::new(),
            unsupported: Vec::new(),
        }
    }

    /// How many catalogue rows were read (with a value or as not set), of how many.
    pub fn coverage(&self) -> (usize, usize) {
        let read = self
            .readings
            .values()
            .filter(|r| matches!(r, Reading::Value(_) | Reading::NotSet))
            .count();
        (read, self.readings.len())
    }

    /// Reads a saved capture: a newer format is refused; every value is checked against `cat`
    /// again, and a setting it does not know is refused.
    pub fn from_json(bytes: &[u8], cat: &Catalogue) -> Result<Self, PlanError> {
        pctwin_types::nesting::check_bytes(bytes, pctwin_types::nesting::Syntax::Json)
            .map_err(|e| PlanError::Damaged(e.to_string()))?;
        let c: Capture =
            serde_json::from_slice(bytes).map_err(|e| PlanError::Damaged(e.to_string()))?;
        if c.format == 0 || c.format > CAPTURE_FORMAT {
            return Err(PlanError::NewerFormat(c.format));
        }
        for (id, r) in &c.readings {
            let row = cat
                .row(id.as_str())
                .ok_or_else(|| PlanError::UnknownSetting(id.as_str().to_string()))?;
            if let Reading::Value(v) = r {
                row.value.check(id.as_str(), v)?;
            }
        }
        Ok(c)
    }
}

/// Where a file the old laptop pointed at is on the new laptop, from the move's item map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Arrival {
    Arrived(String),
    /// Not yet (a cloud placeholder to download first, or still being copied).
    NotYet,
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
    pub managed: BTreeSet<SettingId>,
    /// The person's folders on the new laptop (full paths).
    pub roots: Vec<String>,
    /// Apps installed on the new laptop, by their neutral names.
    pub apps: BTreeSet<String>,
    /// Devices on the new laptop, by name.
    pub devices: BTreeSet<String>,
}

/// The person's answers.
#[derive(Debug, Clone, PartialEq)]
pub enum Choice {
    /// Use this value (one of the options offered).
    Use(SettingValue),
    /// Keep what the new laptop has.
    Keep,
    /// Allow the change (lowering accessibility; an opt-in).
    Allow,
}

pub type Choices = BTreeMap<SettingId, Choice>;

/// Who the settings belong to, as the move mapped people (stable account IDs, never names).
#[derive(Debug, Clone)]
pub struct People<'a> {
    pub old_account: &'a str,
    pub new_account: &'a str,
    pub mapped: &'a [(String, String)],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    Exact,
    Equivalent,
    Guided,
    Unsupported,
    NotApplicable,
}

/// Something a setting waits for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dependency {
    File(String),
    App(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Set it, through the executor's journaled compare-and-write.
    Apply(SettingValue),
    AlreadySet,
    NeedsChoice {
        options: Vec<SettingValue>,
        why: &'static str,
    },
    /// Open the system's own page for the person's choice, then read the result back.
    Handoff(SettingValue),
    /// The person does it; the old value is shown.
    Guided(Option<SettingValue>),
    Privileged(SettingValue),
    OptIn(SettingValue),
    Wait(Dependency),
    Leave(&'static str),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Planned {
    pub id: SettingId,
    pub outcome: Outcome,
    pub action: Action,
    pub old: Option<SettingValue>,
    pub before: Option<SettingValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SettingsPlan {
    pub system: System,
    pub settings: Vec<Planned>,
    pub protected: Vec<(Protected, &'static str)>,
    /// Found on the old laptop with no catalogue row yet.
    pub unsupported: Vec<Found>,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum PlanError {
    #[error(transparent)]
    Value(#[from] ValueError),
    #[error("these settings belong to another person on the new laptop")]
    OwnerMismatch,
    #[error("these settings were read by a newer PCTwin (format {0}); update PCTwin")]
    NewerFormat(u32),
    #[error("PCTwin does not know the setting {0}")]
    UnknownSetting(String),
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
pub const MANAGED_BEFORE: &str = "the old laptop's organisation managed this setting";
pub const DEFAULT_BEFORE: &str = "the old laptop used the system's default";
pub const NOT_READ: &str = "PCTwin could not read this on the old laptop";
pub const OUTSIDE: &str = "the file is not inside your folders on the new laptop";
pub const NOT_MOVED: &str = "the file was not part of the move";
pub const KEPT_STRONGER: &str =
    "the new laptop has a stronger accessibility setting; it is kept unless you choose otherwise";
pub const CHOOSE: &str = "the new laptop cannot hold this exactly; choose what to use";
pub const NO_APP: &str = "this app does not exist on the new laptop's system; choose one";
pub const NO_DEVICE: &str = "this device is not on the new laptop; choose one again";
pub const NOT_THERE: &str = "the new laptop has no such setting";
pub const KEPT_BY_YOU: &str = "you chose to keep the new laptop's setting";

/// Plans every row of the catalogue.
pub fn plan(
    cat: &Catalogue,
    capture: &Capture,
    to: System,
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
    if capture.format == 0 || capture.format > CAPTURE_FORMAT {
        return Err(PlanError::NewerFormat(capture.format));
    }
    for (id, r) in &capture.readings {
        let row = cat
            .row(id.as_str())
            .ok_or_else(|| PlanError::UnknownSetting(id.as_str().to_string()))?;
        if let Reading::Value(v) = r {
            row.value.check(id.as_str(), v)?;
        }
    }
    let mut settings = Vec::new();
    for row in &cat.settings {
        let reading = capture
            .readings
            .get(&row.id)
            .cloned()
            .unwrap_or(Reading::NotBuilt);
        let before = match target.current.get(&row.id) {
            Some(Reading::Value(v)) => Some(v.clone()),
            _ => None,
        };
        let p = One {
            cat,
            row,
            from: capture.system,
            to,
            target,
            files,
            choice: choices.get(&row.id),
            before,
        }
        .plan(&reading);
        if let Action::Apply(v) = &p.action {
            row.value.check(row.id.as_str(), v)?;
        }
        settings.push(p);
    }
    let protected = capture
        .protected
        .iter()
        .map(|p| {
            let why = if p.is_protection_off() {
                PROTECTION_KEPT
            } else {
                APPROVE_AGAIN
            };
            (*p, why)
        })
        .collect();
    Ok(SettingsPlan {
        system: to,
        settings,
        protected,
        unsupported: capture.unsupported.clone(),
    })
}

struct One<'a> {
    cat: &'a Catalogue,
    row: &'a Row,
    from: System,
    to: System,
    target: &'a Target,
    files: &'a dyn PathMap,
    choice: Option<&'a Choice>,
    before: Option<SettingValue>,
}

impl One<'_> {
    fn done(&self, old: &Option<SettingValue>, outcome: Outcome, action: Action) -> Planned {
        Planned {
            id: self.row.id.clone(),
            outcome,
            action,
            old: old.clone(),
            before: self.before.clone(),
        }
    }

    fn plan(&self, reading: &Reading) -> Planned {
        let old = match reading {
            Reading::Value(v) => Some(v.clone()),
            _ => None,
        };
        let row = self.row;
        if row.not_applicable.contains(&self.to) {
            return self.done(&old, Outcome::NotApplicable, Action::Leave(NOT_THERE));
        }
        // The old system keeps this setting where no catalogue place can read it (a Mac's
        // desktop picture, its display scaling): never "could not read". When the new laptop has
        // a page or a place for it, or the setting is the person's step anyway, it is a guided
        // step the person finishes on the new laptop, with nothing claimed as moved.
        if matches!(reading, Reading::NotBuilt)
            && !row.places.contains_key(&self.from)
            && (row.route == Route::Guided
                || row.pages.contains_key(&self.to)
                || row.places.get(&self.to).is_some_and(|p| !p.read_only()))
        {
            return self.done(&old, Outcome::Guided, Action::Guided(None));
        }
        let Some(value) = old.clone() else {
            let why = match reading {
                Reading::Managed => MANAGED_BEFORE,
                Reading::NotSet | Reading::NotApplicable => DEFAULT_BEFORE,
                _ => NOT_READ,
            };
            return self.done(&old, Outcome::Unsupported, Action::Leave(why));
        };
        if self.target.managed.contains(&row.id) {
            return self.done(&old, Outcome::Unsupported, Action::Leave(MANAGED));
        }
        if self.choice == Some(&Choice::Keep) {
            return self.done(&old, Outcome::Exact, Action::Leave(KEPT_BY_YOU));
        }
        let outcome_by_system = if self.from == self.to {
            Outcome::Exact
        } else {
            Outcome::Equivalent
        };
        match row.route {
            Route::Handoff => return self.app(&old, value, Action::Handoff),
            Route::Guided => {
                if row.device {
                    return self.device(&old, value);
                }
                return self.done(&old, Outcome::Guided, Action::Guided(Some(value)));
            }
            Route::Privileged => {
                return self.done(&old, outcome_by_system, Action::Privileged(value));
            }
            Route::OptIn => {
                let action = match self.choice {
                    Some(Choice::Allow) => Action::Guided(Some(value)),
                    _ => Action::OptIn(value),
                };
                return self.done(&old, Outcome::Guided, action);
            }
            Route::Native => {}
        }
        let place = row.places.get(&self.to).filter(|p| !p.read_only());
        // The value to use on the new laptop, before asking whether it can hold it. A file
        // setting the person sets by hand is shown with where its file is on the new laptop.
        let wanted = match (&row.value, &value) {
            (ValueType::FilePath, SettingValue::Text(old_path)) => {
                match self.files.arrival(old_path) {
                    Arrival::Arrived(p) if inside(&p, &self.target.roots) => SettingValue::Text(p),
                    Arrival::Arrived(_) => {
                        return self.done(&old, Outcome::Unsupported, Action::Leave(OUTSIDE));
                    }
                    Arrival::NotYet => {
                        return self.done(
                            &old,
                            Outcome::Exact,
                            Action::Wait(Dependency::File(old_path.clone())),
                        );
                    }
                    Arrival::NotMoved => {
                        return self.done(&old, Outcome::Unsupported, Action::Leave(NOT_MOVED));
                    }
                }
            }
            (ValueType::List { .. }, SettingValue::Text(list)) if row.merge == Merge::AddOnly => {
                let mut out: Vec<String> = match &self.before {
                    Some(SettingValue::Text(t)) if !t.is_empty() => {
                        t.split(',').map(str::to_string).collect()
                    }
                    _ => Vec::new(),
                };
                for item in list.split(',').filter(|i| !i.is_empty()) {
                    if !out.iter().any(|o| o == item) {
                        out.push(item.to_string());
                    }
                }
                SettingValue::Text(out.join(","))
            }
            _ => value.clone(),
        };
        let Some(place) = place else {
            let shown = match row.value {
                ValueType::FilePath => wanted,
                _ => value,
            };
            return self.done(&old, Outcome::Guided, Action::Guided(Some(shown)));
        };
        let held = self.hold(place, &wanted);
        let (outcome, mut new_value) = match held {
            Some(q) if q == wanted => (Outcome::Exact, q),
            Some(q) => (Outcome::Equivalent, q),
            None => {
                // The meaning changes: offer what the new system can hold.
                let options: Vec<SettingValue> = match &row.value {
                    ValueType::Choice { options } => options
                        .iter()
                        .map(|o| SettingValue::Text(o.clone()))
                        .filter(|o| self.hold(place, o).as_ref() == Some(o))
                        .collect(),
                    ValueType::Bool => [false, true]
                        .into_iter()
                        .map(SettingValue::Bool)
                        .filter(|o| self.hold(place, o).as_ref() == Some(o))
                        .collect(),
                    _ => Vec::new(),
                };
                if options.is_empty() {
                    return self.done(&old, Outcome::Guided, Action::Guided(Some(value)));
                }
                match self.choice {
                    Some(Choice::Use(v)) if options.contains(v) => (Outcome::Equivalent, v.clone()),
                    _ => {
                        return self.done(
                            &old,
                            Outcome::Equivalent,
                            Action::NeedsChoice {
                                options,
                                why: CHOOSE,
                            },
                        );
                    }
                }
            }
        };
        if outcome == Outcome::Equivalent
            && let Some(Choice::Use(v)) = self.choice
            && self.hold(place, v).as_ref() == Some(v)
        {
            new_value = v.clone();
        }
        if self.before.as_ref() == Some(&new_value) {
            return self.done(&old, outcome, Action::AlreadySet);
        }
        if row.accessibility
            && lowers(self.before.as_ref(), &new_value)
            && self.choice != Some(&Choice::Allow)
        {
            return self.done(
                &old,
                outcome,
                Action::NeedsChoice {
                    options: vec![new_value],
                    why: KEPT_STRONGER,
                },
            );
        }
        self.done(&old, outcome, Action::Apply(new_value))
    }

    /// What `v` becomes on the new laptop's place, if it can hold it.
    fn hold(&self, place: &Place, v: &SettingValue) -> Option<SettingValue> {
        codec::quantize(self.cat, self.row, place, v).ok()
    }

    /// An app setting (default browser): the app must exist on the new system and be installed.
    fn app(
        &self,
        old: &Option<SettingValue>,
        value: SettingValue,
        then: fn(SettingValue) -> Action,
    ) -> Planned {
        let Some(table) = self
            .row
            .app_table
            .as_ref()
            .and_then(|t| self.cat.tables.get(t))
        else {
            return self.done(old, Outcome::Guided, then(value));
        };
        let SettingValue::Text(name) = &value else {
            return self.done(old, Outcome::Unsupported, Action::Leave(NOT_READ));
        };
        let exists = |n: &str| {
            table
                .lookup("neutral", n, self.to.column(), false)
                .is_some()
        };
        if !exists(name) {
            let options: Vec<SettingValue> = self
                .target
                .apps
                .iter()
                .filter(|a| exists(a))
                .map(|a| SettingValue::Text(a.clone()))
                .collect();
            if let Some(Choice::Use(v)) = self.choice
                && options.contains(v)
            {
                return self.done(old, Outcome::Equivalent, then(v.clone()));
            }
            return self.done(
                old,
                Outcome::Equivalent,
                Action::NeedsChoice {
                    options,
                    why: NO_APP,
                },
            );
        }
        if !self.target.apps.contains(name) {
            return self.done(
                old,
                Outcome::Exact,
                Action::Wait(Dependency::App(name.clone())),
            );
        }
        self.done(old, Outcome::Exact, then(value))
    }

    fn device(&self, old: &Option<SettingValue>, value: SettingValue) -> Planned {
        match &value {
            SettingValue::Text(name) if self.target.devices.contains(name) => {
                self.done(old, Outcome::Guided, Action::Guided(Some(value)))
            }
            _ => self.done(
                old,
                Outcome::Guided,
                Action::NeedsChoice {
                    options: self
                        .target
                        .devices
                        .iter()
                        .map(|d| SettingValue::Text(d.clone()))
                        .collect(),
                    why: NO_DEVICE,
                },
            ),
        }
    }
}

/// Whether `new` is a weaker accessibility setting than `before`.
fn lowers(before: Option<&SettingValue>, new: &SettingValue) -> bool {
    match (before, new) {
        (Some(SettingValue::Int(b)), SettingValue::Int(n)) => n < b,
        (Some(SettingValue::Bool(true)), SettingValue::Bool(false)) => true,
        _ => false,
    }
}

impl Planned {
    /// The executor recipe that applies this setting: one checked `apply-setting` step for the
    /// new laptop's system, versioned by the catalogue revision. `None` unless the action is
    /// [`Action::Apply`].
    pub fn recipe(&self, os: Os, revision: u64) -> Result<Option<Recipe>, PlanError> {
        let Action::Apply(value) = &self.action else {
            return Ok(None);
        };
        let id = RecipeId::new(&format!("pctwin.settings.{}", self.id.as_str()))?;
        let version = u32::try_from(revision).map_err(|_| PlanError::Recipe("revision".into()))?;
        let platforms = [Arch::X86_64, Arch::Aarch64]
            .into_iter()
            .map(|arch| Platform { os, arch })
            .collect();
        Ok(Some(Recipe::new(
            id,
            version,
            platforms,
            vec![Step::ApplySetting {
                key: self.id.as_str().to_string(),
                value: value.clone(),
            }],
        )?))
    }
}

/// The recipe platform OS of a system.
pub fn os_of(system: System) -> Os {
    match system {
        System::Windows => Os::Windows,
        System::Mac => Os::Macos,
        System::Gnome | System::Kde | System::OtherLinux => Os::Linux,
    }
}
