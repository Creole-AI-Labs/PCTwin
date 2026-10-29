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

#[cfg(test)]
mod tests {
    use super::*;
    use pctwin_record::{AppArch, AppMaker, FoundWhere, Owner, RegistryView, SignerId};

    const CODE: &str = "{11111111-2222-3333-4444-555555555555}";

    fn writer() -> FoundApp {
        FoundApp {
            name: "Writer".into(),
            version: Some("3.1".into()),
            publisher: Some("Example Ltd".into()),
            owner: Owner::Shared,
            system: AppSystem::Windows,
            found: FoundWhere::WindowsEntry {
                view: RegistryView::Machine,
                key: "Writer".into(),
                product_code: Some(CODE.into()),
            },
            arch: AppArch::UNKNOWN,
            maker: AppMaker::Windows(Box::new(SignerId {
                organization: Some("Example Ltd".into()),
                country: Some("GB".into()),
                ..Default::default()
            })),
            web: None,
            update_feed: None,
            installers: vec![],
            notes: vec![],
        }
    }

    fn file(rows: &str) -> Catalogue<CompatRow> {
        Catalogue::shipped(&format!(
            r#"{{"format":1,"catalogue":"app-compat","version":1,"rows":[{rows}]}}"#
        ))
    }

    fn row(extra: &str) -> String {
        format!(
            r#"{{"id":"writer-win-win","from":"windows","to":"windows",
                "match":{{"product-code":"{CODE}"}},
                "checked-on":"2026-10-29","how":"lab","version":"3.1",{extra}}}"#
        )
    }

    fn map(pairs: &[(Aspect, AspectResult)]) -> BTreeMap<Aspect, AspectResult> {
        pairs.iter().copied().collect()
    }

    #[test]
    fn the_outcome_is_never_more_than_the_tests_showed() {
        use Aspect::*;
        use AspectResult::*;
        assert_eq!(outcome(&map(&[])), Outcome::NotTested);
        // An install that was not tested is "not tested", whatever else was.
        assert_eq!(
            outcome(&map(&[(Settings, Works), (Launch, Works)])),
            Outcome::NotTested
        );
        assert_eq!(
            outcome(&map(&[(Install, NotApplicable)])),
            Outcome::NotTested
        );
        assert_eq!(
            outcome(&map(&[(Install, NotSupported)])),
            Outcome::NotSupported
        );
        assert_eq!(
            outcome(&map(&[(Install, NotSupported), (Launch, Works)])),
            Outcome::NotSupported
        );
        assert_eq!(outcome(&map(&[(Install, Works)])), Outcome::Works);
        assert_eq!(
            outcome(&map(&[(Install, Works), (Settings, NotApplicable)])),
            Outcome::Works
        );
        assert_eq!(
            outcome(&map(&[(Install, Works), (Launch, Guided)])),
            Outcome::WorksWithSteps
        );
        assert_eq!(outcome(&map(&[(Install, Guided)])), Outcome::WorksWithSteps);
        assert_eq!(
            outcome(&map(&[
                (Install, Works),
                (Launch, Guided),
                (Undo, NotSupported)
            ])),
            Outcome::Partly
        );
        assert_eq!(
            outcome(&map(&[(Install, Works), (DataImport, NotSupported)])),
            Outcome::Partly
        );
    }

    #[test]
    fn an_app_with_no_row_is_not_tested_and_never_presumed_to_work() {
        let t = tested(shipped(), &writer(), AppSystem::Windows);
        assert_eq!(t.outcome, Outcome::NotTested);
        assert_eq!(t.aspects.len(), Aspect::ALL.len());
        assert!(
            t.aspects
                .iter()
                .all(|a| a.result == AspectResult::NotTested)
        );
        assert_eq!((t.checked_on, t.how, t.version), (None, None, None));
        assert!(
            shipped().report().refused.is_none(),
            "the shipped file reads"
        );
    }

    /// ADOPTION change 7: a new app added only as a catalogue row works end to end, with no
    /// code change.
    #[test]
    fn a_row_added_as_data_alone_shows_its_real_tested_outcome() {
        let cat = file(&row(
            r#""tested":{"install":"works","settings":"works","launch":"guided","undo":"works"},
               "steps":["sign-in"]"#,
        ));
        assert!(cat.report().skipped.is_empty(), "{:?}", cat.report());
        let t = tested(&cat, &writer(), AppSystem::Windows);
        assert_eq!(t.outcome, Outcome::WorksWithSteps);
        let of = |a| t.aspects.iter().find(|l| l.aspect == a).unwrap().result;
        assert_eq!(of(Aspect::Install), AspectResult::Works);
        assert_eq!(of(Aspect::Launch), AspectResult::Guided);
        // Left out of the row: not tested, never presumed.
        assert_eq!(of(Aspect::DataImport), AspectResult::NotTested);
        assert_eq!(t.steps, vec![GuidedStep::SignIn]);
        assert_eq!(t.checked_on.as_deref(), Some("2026-10-29"));
        assert_eq!(t.how, Some(TestedHow::Lab));
        assert_eq!(t.version.as_deref(), Some("3.1"));
        // Another pair of systems is another test.
        assert_eq!(
            tested(&cat, &writer(), AppSystem::Mac).outcome,
            Outcome::NotTested
        );
        let mut mac = writer();
        mac.system = AppSystem::Mac;
        assert_eq!(
            tested(&cat, &mac, AppSystem::Windows).outcome,
            Outcome::NotTested
        );
        // Another app (another product code, a similar name) is not this row's.
        let mut other = writer();
        other.found = FoundWhere::WindowsEntry {
            view: RegistryView::Machine,
            key: "Writer".into(),
            product_code: Some("{99999999-2222-3333-4444-555555555555}".into()),
        };
        assert_eq!(
            tested(&cat, &other, AppSystem::Windows).outcome,
            Outcome::NotTested
        );
    }

    #[test]
    fn a_row_that_claims_more_than_a_test_can_show_is_skipped() {
        let bad = [
            // No identity: a name alone never matches.
            r#"{"id":"x","from":"windows","to":"windows","match":{},"tested":{"install":"works"},
                "checked-on":"2026-10-29","how":"lab","version":"1"}"#
                .to_string(),
            // An aspect written as "not tested".
            row(r#""tested":{"install":"works","undo":"not-tested"}"#),
            // Nothing tested.
            row(r#""tested":{}"#),
            // A guided part with no steps, and steps with no guided part.
            row(r#""tested":{"install":"guided"}"#),
            row(r#""tested":{"install":"works"},"steps":["restart"]"#),
            // No date, or not a date.
            row(r#""tested":{"install":"works"}"#).replace("2026-10-29", "29/10/2026"),
            row(r#""tested":{"install":"works"}"#).replace("2026-10-29", "2026-13-01"),
            row(r#""tested":{"install":"works"}"#).replace("2026-10-29", "2026-10-32"),
            row(r#""tested":{"install":"works"}"#).replace("2026-10-29", "2026x10-29"),
            row(r#""tested":{"install":"works"}"#).replace("2026-10-29", "20x6-10-29"),
            // No version.
            row(r#""tested":{"install":"works"}"#)
                .replace(r#""version":"3.1""#, r#""version":" ""#),
            // A row ID with a slash.
            row(r#""tested":{"install":"works"}"#).replace("writer-win-win", "a/b"),
            row(r#""tested":{"install":"works"}"#).replace("writer-win-win", ""),
        ];
        for b in &bad {
            let cat = file(b);
            assert!(cat.rows().is_empty(), "accepted: {b}");
        }
        let good = file(&row(r#""tested":{"install":"works"}"#));
        assert_eq!(good.rows().len(), 1, "{:?}", good.report());
        assert!(date_ok("2026-01-01") && date_ok("2026-12-31"));
        assert!(!date_ok("2026-00-10") && !date_ok("2026-10-00") && !date_ok("2026-10-1"));
    }

    #[test]
    fn two_rows_for_one_app_and_pair_are_both_skipped() {
        let a = row(r#""tested":{"install":"works"}"#);
        let b = a.replace("writer-win-win", "writer-again");
        let cat = file(&format!("{a},{b}"));
        assert!(cat.rows().is_empty());
    }
}
