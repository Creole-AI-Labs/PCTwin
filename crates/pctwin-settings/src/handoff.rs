//! Settings the system keeps for the person to choose (the default browser, file handlers):
//! PCTwin opens the system's own page and reads the result back; it never edits the protected
//! choice itself (T24-011). A page that opened is not a setting restored (T24-012).
//!
//! Windows protects these choices with a hash and, since February 2024, the UCPD driver; macOS
//! always shows its own confirmation. Opening the page is done by the app (the system's own
//! opener); this module builds the page address from the catalogue row and judges the result.

use pctwin_recipe::SettingValue;
use pctwin_types::{ItemResult, ItemStatus, Reason};

use crate::catalogue::{Catalogue, Row, System};
use crate::registry::Hive;

/// The page to open for `row` on `system`, naming the app when the row's page can (Windows'
/// Default apps page, `ms-settings:defaultapps?registeredAppUser=<name>`). `registered` lists
/// the names in the registry's registered applications; the app's name there must start with
/// the catalogue's name for it (Firefox registers `Firefox-<install hash>`).
pub fn page(
    cat: &Catalogue,
    row: &Row,
    system: System,
    app: Option<&SettingValue>,
    registered: &[String],
) -> Option<String> {
    let page = row.pages.get(&system)?;
    let Some(param) = &page.app_param else {
        return Some(page.uri.clone());
    };
    let named = (|| {
        let SettingValue::Text(app) = app? else {
            return None;
        };
        let base = cat
            .tables
            .get(&param.table)?
            .lookup("neutral", app, &param.column, false)?;
        let found = registered.iter().find(|r| {
            r.as_str() == base
                || r.strip_prefix(base)
                    .is_some_and(|rest| rest.starts_with('-'))
        })?;
        Some(format!("{}?{}={}", page.uri, param.name, encode(found)))
    })();
    // Without the app's own name, the general page.
    Some(named.unwrap_or_else(|| page.uri.clone()))
}

/// Percent-encodes a query value (everything but unreserved characters).
fn encode(s: &str) -> String {
    s.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
                char::from(b).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

/// The registered application names on Windows (the person's and the machine's).
pub fn registered_apps(user: Option<&dyn Hive>, machine: Option<&dyn Hive>) -> Vec<String> {
    let mut out = Vec::new();
    for (h, key) in [
        (user, "Software\\RegisteredApplications"),
        (machine, "SOFTWARE\\RegisteredApplications"),
    ] {
        if let Some(h) = h {
            out.extend(
                h.names(key)
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|n| !n.is_empty()),
            );
        }
    }
    out
}

/// How a handoff ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Handoff {
    /// Read back: the system now has the person's old choice.
    Verified,
    /// The system cannot be read here; the person said they chose it. Shown as their word, not
    /// as PCTwin's check.
    PersonConfirmed,
    /// Not done yet (the page opened, nothing chosen, or something else chosen).
    NotYet,
}

/// Judges a handoff from the value read back after the person returns.
pub fn judge(
    wanted: &SettingValue,
    readback: Result<Option<SettingValue>, String>,
    person_says_done: bool,
) -> Handoff {
    match readback {
        Ok(Some(v)) if &v == wanted => Handoff::Verified,
        Ok(_) => Handoff::NotYet,
        Err(_) if person_says_done => Handoff::PersonConfirmed,
        Err(_) => Handoff::NotYet,
    }
}

/// The honest result: verified is converted; anything else is the person's step still open.
pub fn item_result(h: &Handoff) -> ItemResult {
    match h {
        Handoff::Verified => ItemResult::done(ItemStatus::Converted)
            .unwrap_or(ItemResult::because(Reason::FinishByHand)),
        Handoff::PersonConfirmed | Handoff::NotYet => ItemResult::because(Reason::FinishByHand),
    }
}
