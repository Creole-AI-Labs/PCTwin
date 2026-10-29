//! The compatibility record (Task List 2.9): for an app and a pair of systems (the old laptop's
//! and the new one's), what was really tested (installing it, importing its data, its settings,
//! starting it, undoing it), when, and how. The person sees the real supported outcome, with any
//! guided steps.
//!
//! The record is **data** (`catalogues/app-compat.json`, ADOPTION change 7): a signed catalogue
//! read through `pctwin-catalogue`, one row per app and system pair, matched by the app's own
//! identity exactly as `app-routes` rows are (never a name alone). A row only says what a test
//! showed; it never decides what is found or what moves (ADOPTION change 8). An app with no row
//! is "not tested", never presumed to work, and still moves by the general rules.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use std::sync::OnceLock;

use pctwin_catalogue::{Catalogue, Row};
use pctwin_record::{AppSystem, FoundApp};

use crate::routes::rows::RowMatch;

/// One part of an app's move that a test covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Aspect {
    Install,
    DataImport,
    Settings,
    Launch,
    Undo,
}

impl Aspect {
    /// Every aspect, in the order the person is shown them.
    pub const ALL: [Aspect; 5] = [
        Aspect::Install,
        Aspect::DataImport,
        Aspect::Settings,
        Aspect::Launch,
        Aspect::Undo,
    ];
}

/// What a test of one aspect showed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AspectResult {
    /// It worked by itself.
    Works,
    /// It worked with the person's guided steps.
    Guided,
    /// It did not work: PCTwin does not do this part for this app on these systems.
    NotSupported,
    /// The app has no such part (an app with no data to import).
    NotApplicable,
    /// Not tested: never presumed to work.
    NotTested,
}

/// A step the person takes, that a test needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GuidedStep {
    /// Sign in to the app.
    SignIn,
    /// Enter its licence key, or activate it.
    Licence,
    /// Import its data with the app's own import.
    ImportInApp,
    /// Point it at its files again.
    Relink,
    /// Restart the laptop.
    Restart,
}

/// How a test was done.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TestedHow {
    /// PCTwin's own test lab on clean systems (CI runners).
    Lab,
    /// A real move between two real laptops.
    RealLaptops,
}

/// The one outcome the person is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    /// Every part tested works by itself.
    Works,
    /// Every part tested works, some with the person's steps.
    WorksWithSteps,
    /// Some parts work, some do not.
    Partly,
    /// PCTwin cannot install it on these systems.
    NotSupported,
    /// No test of this app on these systems: never presumed to work.
    NotTested,
}

/// One `app-compat` row: what a test of one app between two systems showed.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CompatRow {
    pub id: String,
    /// The old laptop's system.
    pub from: AppSystem,
    /// The new laptop's system.
    pub to: AppSystem,
    #[serde(rename = "match")]
    pub matches: RowMatch,
    /// What each aspect showed; an aspect left out was not tested.
    pub tested: BTreeMap<Aspect, AspectResult>,
    #[serde(default)]
    pub steps: Vec<GuidedStep>,
    /// When it was tested (YYYY-MM-DD).
    pub checked_on: String,
    pub how: TestedHow,
    /// The app version the test used.
    pub version: String,
}

/// One aspect and what a test of it showed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct AspectLine {
    pub aspect: Aspect,
    pub result: AspectResult,
}

/// What the person is shown of an app's compatibility record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tested {
    pub outcome: Outcome,
    /// Every aspect, each with what a test showed (`not-tested` where none did).
    pub aspects: Vec<AspectLine>,
    pub steps: Vec<GuidedStep>,
    /// When, how and with which version (`None` when not tested).
    pub checked_on: Option<String>,
    pub how: Option<TestedHow>,
    pub version: Option<String>,
}

impl Tested {
    /// No test of this app on these systems.
    pub fn not_tested() -> Self {
        Tested {
            outcome: Outcome::NotTested,
            aspects: Aspect::ALL
                .iter()
                .map(|a| AspectLine {
                    aspect: *a,
                    result: AspectResult::NotTested,
                })
                .collect(),
            steps: Vec::new(),
            checked_on: None,
            how: None,
            version: None,
        }
    }
}

/// The one outcome of a row's aspects: installing decides whether it is supported at all; a
/// part that did not work makes it "partly"; a guided part makes it "works with steps". An
/// install that was not tested is "not tested", whatever else was.
pub fn outcome(tested: &BTreeMap<Aspect, AspectResult>) -> Outcome {
    let of = |a| tested.get(&a).copied().unwrap_or(AspectResult::NotTested);
    match of(Aspect::Install) {
        AspectResult::NotSupported => return Outcome::NotSupported,
        AspectResult::NotTested | AspectResult::NotApplicable => return Outcome::NotTested,
        AspectResult::Works | AspectResult::Guided => {}
    }
    let results: Vec<AspectResult> = Aspect::ALL.iter().map(|a| of(*a)).collect();
    if results.contains(&AspectResult::NotSupported) {
        Outcome::Partly
    } else if results.contains(&AspectResult::Guided) {
        Outcome::WorksWithSteps
    } else {
        Outcome::Works
    }
}

/// Whether `s` is a date written YYYY-MM-DD.
fn date_ok(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
        && matches!(s[5..7].parse::<u8>(), Ok(1..=12))
        && matches!(s[8..10].parse::<u8>(), Ok(1..=31))
}

impl Row for CompatRow {
    const CATALOGUE: &'static str = "app-compat";
    const FIELDS: &'static [&'static str] = &[
        "id",
        "from",
        "to",
        "match",
        "tested",
        "steps",
        "checked-on",
        "how",
        "version",
    ];

    fn check(&self) -> Result<(), String> {
        if self.id.is_empty()
            || self.id.len() > 64
            || !self
                .id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
        {
            return Err("a row ID is short letters, digits and `.-_`".into());
        }
        crate::routes::rows::check_match(&self.matches)?;
        if !date_ok(&self.checked_on) {
            return Err("a row says when it was tested, as YYYY-MM-DD".into());
        }
        if self.version.trim().is_empty() || !pctwin_record::apps::text_ok(&self.version, 64) {
            return Err("a row names the version tested".into());
        }
        if self.tested.is_empty() {
            return Err("a row says what was tested".into());
        }
        if self.tested.values().any(|r| *r == AspectResult::NotTested) {
            return Err("an aspect not tested is left out, never written as tested".into());
        }
        let guided = self.tested.values().any(|r| *r == AspectResult::Guided);
        if guided == self.steps.is_empty() {
            return Err(
                "guided aspects name their steps, and steps belong to guided aspects".into(),
            );
        }
        Ok(())
    }

    fn identity(&self) -> Option<String> {
        Some(format!("{:?}>{:?}:{:?}", self.from, self.to, self.matches))
    }
}

/// The rows compiled into this build (none yet: a row is added only from a real test's
/// evidence, never presumed).
pub fn shipped() -> &'static Catalogue<CompatRow> {
    static ROWS: OnceLock<Catalogue<CompatRow>> = OnceLock::new();
    ROWS.get_or_init(|| Catalogue::shipped(include_str!("../catalogues/app-compat.json")))
}

/// What tests of `app` (found on the old laptop) on a laptop with system `to` showed, by the row
/// for exactly this app and this pair of systems; with none, "not tested".
pub fn tested(rows: &Catalogue<CompatRow>, app: &FoundApp, to: AppSystem) -> Tested {
    let Some(row) = rows.rows().iter().find(|r| {
        r.from == app.system
            && r.to == to
            && crate::routes::rows::identity_matches(app, r.from, &r.matches)
    }) else {
        return Tested::not_tested();
    };
    Tested {
        outcome: outcome(&row.tested),
        aspects: Aspect::ALL
            .iter()
            .map(|a| AspectLine {
                aspect: *a,
                result: row
                    .tested
                    .get(a)
                    .copied()
                    .unwrap_or(AspectResult::NotTested),
            })
            .collect(),
        steps: row.steps.clone(),
        checked_on: Some(row.checked_on.clone()),
        how: Some(row.how),
        version: Some(row.version.clone()),
    }
}
