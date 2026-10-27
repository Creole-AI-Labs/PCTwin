//! Privileged setting rows (Phase 2 adoption, change 7, its last and most careful step).
//!
//! The settings the helper may apply with an administrator's rights stay inside an area pinned
//! in code: each row can only point at one of the compiled mechanisms ([`Mechanism`], the
//! adapters behind [`super::actions::NamedSetting`]). Security settings (the administrator
//! prompt, the firewall, antivirus exclusions, remote access, policies) have no mechanism, so no
//! row can reach them: the deny region and the fixed action kinds stay code.
//!
//! A row adds a name to show and the oldest helper that may act on it, and needs the stricter,
//! privileged signature ([`pctwin_catalogue::Trust::Privileged`]). Which key signs privileged
//! rows is not decided yet (it belongs with the TUF/tough release keys), so the helper uses only
//! the rows compiled into it ([`shipped_setting_rows`]); this module is the row shape and the
//! loader, ready for that decision. The broker's schema ([`super::actions::NamedSetting`]) is
//! unchanged.

use std::sync::OnceLock;

use pctwin_catalogue::{Catalogue, Row, Trust};
use serde::Deserialize;

use super::actions::NamedSetting;

/// The compiled mechanisms a privileged row may point at. Adding one is a reviewed code change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Mechanism {
    TimeZone,
    Hibernate,
}

impl Mechanism {
    /// The mechanism a setting of the broker's schema uses.
    pub fn of(setting: &NamedSetting) -> Self {
        match setting {
            NamedSetting::TimeZone(_) => Mechanism::TimeZone,
            NamedSetting::Hibernate(_) => Mechanism::Hibernate,
        }
    }
}

/// A helper version, `major.minor.patch`.
pub type Version = (u32, u32, u32);

/// Reads `1.2.3`.
pub fn parse_version(text: &str) -> Option<Version> {
    let mut parts = text.split('.');
    let mut next = || -> Option<u32> {
        let p = parts.next()?;
        (!p.is_empty() && p.len() <= 9 && p.bytes().all(|b| b.is_ascii_digit()))
            .then(|| p.parse().ok())
            .flatten()
    };
    let v = (next()?, next()?, next()?);
    parts.next().is_none().then_some(v)
}

/// This helper's own version.
pub fn this_helper() -> Version {
    parse_version(env!("CARGO_PKG_VERSION")).unwrap_or((0, 0, 0))
}

/// One row of the `privileged-settings` catalogue.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SettingRow {
    pub mechanism: Mechanism,
    /// Its name, to show.
    pub name: String,
    /// The oldest helper that may act on it.
    pub min_helper: String,
}

impl Row for SettingRow {
    const CATALOGUE: &'static str = "privileged-settings";
    const FIELDS: &'static [&'static str] = &["mechanism", "name", "min-helper"];
    const TRUST: Trust = Trust::Privileged;
    fn check(&self) -> Result<(), String> {
        let Some(min) = parse_version(&self.min_helper) else {
            return Err("a minimum helper version is major.minor.patch".into());
        };
        // Part of loading (fourth review): a row this helper is too old for is never loaded.
        if min > this_helper() {
            return Err("this row needs a newer helper".into());
        }
        if self.name.trim().is_empty()
            || self.name.len() > 64
            || self.name.chars().any(char::is_control)
        {
            return Err("a name is 1 to 64 plain characters".into());
        }
        Ok(())
    }
}

/// The rows compiled into this helper (the only ones it uses until the privileged signing key is
/// decided).
pub fn shipped_setting_rows() -> &'static Catalogue<SettingRow> {
    static ROWS: OnceLock<Catalogue<SettingRow>> = OnceLock::new();
    ROWS.get_or_init(|| {
        Catalogue::shipped(include_str!("../../catalogues/privileged-settings.json"))
    })
}

/// Why a setting may not be applied by this helper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotAllowed {
    /// No row allows its mechanism.
    NoRow,
    /// Its row needs a newer helper.
    HelperTooOld,
}

/// The row that lets `helper` apply `setting`. A mechanism with no row, or a row needing a newer
/// helper, is refused: rows can only allow what the code already pins, never more.
pub fn allowed<'c>(
    rows: &'c Catalogue<SettingRow>,
    setting: &NamedSetting,
    helper: Version,
) -> Result<&'c SettingRow, NotAllowed> {
    let mechanism = Mechanism::of(setting);
    let reading = rows.read([mechanism], |m, row| row.mechanism == *m);
    let Some((_, row)) = reading.known.into_iter().next() else {
        return Err(NotAllowed::NoRow);
    };
    match parse_version(&row.min_helper) {
        Some(min) if min <= helper => Ok(row),
        _ => Err(NotAllowed::HelperTooOld),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::actions::TimeZoneName;
    use pctwin_catalogue::{CatalogueError, test_key};

    #[test]
    fn the_compiled_rows_allow_exactly_the_compiled_mechanisms() {
        let rows = shipped_setting_rows();
        assert!(rows.report().is_clean(), "{:?}", rows.report());
        let tz = NamedSetting::TimeZone(TimeZoneName::parse("UTC").unwrap());
        assert_eq!(allowed(rows, &tz, this_helper()).unwrap().name, "Time zone");
        assert!(allowed(rows, &NamedSetting::Hibernate(true), this_helper()).is_ok());
    }

    #[test]
    fn versions_are_read_exactly() {
        assert_eq!(parse_version("0.1.0"), Some((0, 1, 0)));
        assert_eq!(parse_version("10.20.30"), Some((10, 20, 30)));
        for bad in [
            "",
            "1",
            "1.2",
            "1.2.3.4",
            "1..3",
            "a.b.c",
            "1.2.-3",
            "1.2.1234567890",
        ] {
            assert_eq!(parse_version(bad), None, "{bad}");
        }
        assert_eq!(
            this_helper(),
            parse_version(env!("CARGO_PKG_VERSION")).unwrap()
        );
    }

    /// A delivered row needs the privileged key, can only point at a compiled mechanism, and can
    /// demand a newer helper; with no row a setting is refused.
    #[test]
    fn delivered_rows_need_the_privileged_key_and_stay_inside_the_pinned_area() {
        let bytes = br#"{"format":1,"catalogue":"privileged-settings","version":2,"rows":[
            {"mechanism":"hibernate","name":"Hibernate","min-helper":"9.0.0"},
            {"mechanism":"firewall","name":"Firewall","min-helper":"0.1.0"},
            {"mechanism":"time-zone","name":"Time zone","min-helper":"one"},
            {"mechanism":"time-zone","name":"","min-helper":"0.1.0"},
            {"mechanism":"time-zone","name":"Clock","min-helper":"0.1.0","command":"x"}
        ]}"#;
        let ordinary = test_key::sign(bytes, "privileged-settings", 2);
        assert_eq!(
            Catalogue::<SettingRow>::signed(
                bytes,
                &ordinary,
                &test_key::keys(),
                &pctwin_catalogue::MemoryMarks::at(1)
            )
            .unwrap_err(),
            CatalogueError::NeedsPrivilegedKey
        );
        let sig = test_key::sign_privileged(bytes, "privileged-settings", 2);
        let rows = Catalogue::<SettingRow>::signed(
            bytes,
            &sig,
            &test_key::privileged_keys(),
            &pctwin_catalogue::MemoryMarks::at(1),
        )
        .unwrap();
        // The row needing helper 9.0.0 is not loaded by this helper at all (fourth review: the
        // minimum helper version is part of loading); the other four are not rows.
        assert_eq!(rows.rows().len(), 0);
        assert_eq!(rows.report().skipped.len(), 5);
        assert!(
            rows.report()
                .skipped
                .iter()
                .any(|s| s.row == 0 && s.why == "this row needs a newer helper")
        );
        let hib = NamedSetting::Hibernate(false);
        assert_eq!(allowed(&rows, &hib, (9, 0, 0)), Err(NotAllowed::NoRow));
        // A row this helper is new enough for loads, and `allowed` still checks the version.
        let ok = br#"{"format":1,"catalogue":"privileged-settings","version":3,"rows":[
            {"mechanism":"hibernate","name":"Hibernate","min-helper":"0.0.1"}
        ]}"#;
        let sig = test_key::sign_privileged(ok, "privileged-settings", 3);
        let rows = Catalogue::<SettingRow>::signed(
            ok,
            &sig,
            &test_key::privileged_keys(),
            &pctwin_catalogue::MemoryMarks::at(2),
        )
        .unwrap();
        assert_eq!(rows.rows().len(), 1);
        assert!(allowed(&rows, &hib, this_helper()).is_ok());
        assert_eq!(
            allowed(&rows, &hib, (0, 0, 0)),
            Err(NotAllowed::HelperTooOld)
        );
        let tz = NamedSetting::TimeZone(TimeZoneName::parse("UTC").unwrap());
        assert_eq!(allowed(&rows, &tz, this_helper()), Err(NotAllowed::NoRow));
    }
}
