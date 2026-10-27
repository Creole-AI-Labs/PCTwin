//! The `special` catalogue: what PCTwin knows about some apps' data, as signed data that only
//! enriches what discovery found (ADOPTION changes 5, 7 and 8).
//!
//! A row names an app data folder (system, root kind, folder) and may add:
//! - a friendly name;
//! - where the same app keeps the same data on another system (`elsewhere`): the cross-system
//!   translation, without which data moved between different systems is kept on the new laptop
//!   but not handed to the app there;
//! - typed path fix-ups for files that hold absolute paths ([`Fixup`]), never a free-form text
//!   replacement;
//! - the app's own export route ([`NativeRoute`]): its export screen, the maker's page, or its
//!   own export tool with typed arguments (never a shell string);
//! - the maker's page for signing in again or relinking;
//! - known exceptions, which can only make handling stricter, never looser ([`Exception`]);
//! - the date it was last checked against the maker's own documentation.
//!
//! With no rows at all, everything found is still classified by its content, and either moves,
//! is held back with a reason, or is reported "found, not moved". A row never decides what is
//! found, and never makes something portable that the content rules hold back.

use pctwin_catalogue::Row;
use serde::Deserialize;

use crate::component::Custody;
use crate::roots::{AppDataFolder, RootKind, System};

/// Which systems a row is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Systems {
    Windows,
    Mac,
    Linux,
    Any,
}

impl Systems {
    pub fn covers(self, s: System) -> bool {
        matches!(
            (self, s),
            (Systems::Any, _)
                | (Systems::Windows, System::Windows)
                | (Systems::Mac, System::Mac)
                | (Systems::Linux, System::Linux)
        )
    }
}

/// A place on another system where the same app keeps the same data.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Place {
    pub system: System,
    pub root: RootKind,
    pub folder: String,
}

/// A typed change to a file's paths when it lands (the plan's "typed transformations").
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind", deny_unknown_fields)]
pub enum Fixup {
    /// In INI files of this folder, the value of `key` is a path to a folder that moved with
    /// it. When the line `flag`=1 says the path is relative, nothing changes; otherwise it is
    /// rewritten to the same place under the landed folder.
    IniPath { key: String, flag: Option<String> },
}

/// One argument of an app's own export tool.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind", content = "value")]
pub enum Arg {
    /// A fixed switch, such as `--export` (letters, digits and `-` only, starting with `-`).
    Switch(String),
    /// The file the export is written to: a PCTwin-owned, private place chosen at run time.
    OutputFile,
}

/// The app's own export route.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind", deny_unknown_fields)]
pub enum NativeRoute {
    /// Open the app, as the person, at its own export screen (the person exports).
    OpenApp,
    /// Open the maker's page about exporting, in the person's browser.
    OpenPage { url: String },
    /// Run the app's own export tool, as the person: `tool` is the tool's file name, found in
    /// the installed app's own folder and checked to be signed by the app's maker first.
    Tool { tool: String, args: Vec<Arg> },
}

/// A known exception: files of this folder matching `members` are handled as `custody`, which
/// must be stricter than what the content rules decided (never looser).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exception {
    /// A pattern over the file's path inside the folder (`*` any text; case ignored).
    pub members: String,
    pub custody: Custody,
    pub why: String,
}

/// One row of the `special` catalogue.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SpecialRow {
    pub system: Systems,
    pub root: RootKind,
    /// The app data folder, as found (one or two names, `/` between them); compared ignoring
    /// case.
    pub folder: String,
    #[serde(default)]
    pub name: Option<String>,
    /// When the row was last checked against the maker's own documentation (`YYYY-MM-DD`).
    pub checked_on: String,
    #[serde(default)]
    pub elsewhere: Vec<Place>,
    #[serde(default)]
    pub fixups: Vec<Fixup>,
    #[serde(default)]
    pub export: Option<NativeRoute>,
    /// The maker's page for signing in again or relinking.
    #[serde(default)]
    pub relink: Option<String>,
    #[serde(default)]
    pub exceptions: Vec<Exception>,
}

/// How strict a class is: an exception may only raise it.
pub fn strictness(c: Custody) -> u8 {
    match c {
        Custody::Portable => 0,
        Custody::NeedsExport => 1,
        Custody::ServiceHeld => 2,
        Custody::MachineBound => 3,
        Custody::Unresolved => 4,
    }
}

fn date_ok(d: &str) -> bool {
    let b = d.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    let num = |r: std::ops::Range<usize>| d[r].parse::<u32>().ok();
    matches!(
        (num(0..4), num(5..7), num(8..10)),
        (Some(2020..=2100), Some(1..=12), Some(1..=31))
    )
}

fn folder_ok(f: &str) -> bool {
    let parts: Vec<&str> = f.split('/').collect();
    !f.is_empty()
        && f.len() <= 255
        && parts.len() <= 2
        && parts
            .iter()
            .all(|p| !p.is_empty() && *p != "." && *p != ".." && !p.contains(['\\', ':', '\0']))
}

fn page_ok(u: &str) -> bool {
    u.strip_prefix("https://")
        .is_some_and(|rest| !rest.is_empty() && !rest.contains(char::is_whitespace))
}

impl Row for SpecialRow {
    const CATALOGUE: &'static str = "special";
    const FIELDS: &'static [&'static str] = &[
        "system",
        "root",
        "folder",
        "name",
        "checked_on",
        "elsewhere",
        "fixups",
        "export",
        "relink",
        "exceptions",
    ];

    fn check(&self) -> Result<(), String> {
        if !folder_ok(&self.folder) {
            return Err("the folder must be one or two plain names".into());
        }
        if !date_ok(&self.checked_on) {
            return Err("checked_on must be a date (YYYY-MM-DD)".into());
        }
        if self
            .name
            .as_ref()
            .is_some_and(|n| n.is_empty() || n.len() > 100)
        {
            return Err("a name is 1 to 100 characters".into());
        }
        if self
            .elsewhere
            .iter()
            .any(|p| !folder_ok(&p.folder) || p.root.is_cache())
        {
            return Err("a place elsewhere must be a folder in an app data root".into());
        }
        for f in &self.fixups {
            match f {
                Fixup::IniPath { key, flag } => {
                    let plain = |k: &str| {
                        !k.is_empty()
                            && k.len() <= 64
                            && k.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                    };
                    if !plain(key) || flag.as_deref().is_some_and(|f| !plain(f)) {
                        return Err("a fix-up names plain keys".into());
                    }
                }
            }
        }
        match &self.export {
            Some(NativeRoute::OpenPage { url }) if !page_ok(url) => {
                return Err("a page must be an https address".into());
            }
            Some(NativeRoute::Tool { tool, args }) => {
                let plain_file = !tool.is_empty()
                    && tool.len() <= 64
                    && tool
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
                    && !tool.starts_with('.');
                if !plain_file {
                    return Err("an export tool is named by its plain file name".into());
                }
                let switch_ok = |s: &str| {
                    s.len() >= 2
                        && s.len() <= 64
                        && s.starts_with('-')
                        && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
                };
                if args
                    .iter()
                    .any(|a| matches!(a, Arg::Switch(s) if !switch_ok(s)))
                {
                    return Err("an export switch is letters, digits and dashes".into());
                }
                if args.iter().filter(|a| **a == Arg::OutputFile).count() != 1 {
                    return Err("an export tool writes exactly one output file".into());
                }
            }
            _ => {}
        }
        if self.relink.as_deref().is_some_and(|u| !page_ok(u)) {
            return Err("a page must be an https address".into());
        }
        for e in &self.exceptions {
            if e.members.is_empty() || e.why.is_empty() || e.why.len() > 300 {
                return Err("an exception names its files and says why".into());
            }
            if e.custody == Custody::Portable {
                return Err("an exception can only make handling stricter".into());
            }
        }
        Ok(())
    }

    fn identity(&self) -> Option<String> {
        Some(format!(
            "{:?}/{:?}/{}",
            self.system,
            self.root,
            self.folder.to_lowercase()
        ))
    }
}

impl SpecialRow {
    /// Whether this row is about `folder` found on `system`.
    pub fn matches(&self, system: System, folder: &AppDataFolder) -> bool {
        self.system.covers(system)
            && self.root == folder.root
            && self.folder.eq_ignore_ascii_case(&folder.rel)
    }

    /// The custody for a file at `rel` (inside the folder) whose content rules gave `found`: an
    /// exception's stricter class, with its reason, or `found` unchanged.
    pub fn tighten(&self, rel: &str, found: Custody) -> (Custody, Option<&str>) {
        for e in &self.exceptions {
            if pctwin_settings::apps::glob(&e.members.to_lowercase(), &rel.to_lowercase())
                && strictness(e.custody) > strictness(found)
            {
                return (e.custody, Some(&e.why));
            }
        }
        (found, None)
    }
}

/// The shipped `special` catalogue (compiled in, covered by the app's own signature). It holds no
/// rows yet: everything is found and classified by content alone, and rows arrive through the
/// signed catalogue channel as each app's documentation is checked.
pub const SHIPPED: &str = r#"{"format": 1, "catalogue": "special", "version": 1, "rows": []}"#;

#[cfg(test)]
mod tests {
    use super::*;
    use pctwin_catalogue::Catalogue;

    fn load(rows: &str) -> Catalogue<SpecialRow> {
        Catalogue::shipped(&format!(
            r#"{{"format": 1, "catalogue": "special", "version": 2, "rows": [{rows}]}}"#
        ))
    }

    #[test]
    fn the_shipped_catalogue_reads_and_is_empty() {
        let c = Catalogue::<SpecialRow>::shipped(SHIPPED);
        assert!(c.report().is_clean(), "{:?}", c.report());
        assert!(c.rows().is_empty());
    }

    #[test]
    fn rows_are_checked_and_bad_ones_skipped_with_a_reason() {
        let good = r#"{"system":"windows","root":"roaming","folder":"Synthetic Notes","name":"Synthetic Notes",
            "checked_on":"2026-10-27",
            "elsewhere":[{"system":"mac","root":"application-support","folder":"Synthetic Notes"}],
            "fixups":[{"kind":"ini-path","key":"Path","flag":"IsRelative"}],
            "export":{"kind":"tool","tool":"notes-export","args":[{"kind":"switch","value":"--all"},{"kind":"output-file"}]},
            "relink":"https://example.org/relink",
            "exceptions":[{"members":"tokens/*","custody":"D","why":"sign-in tokens"}]}"#;
        let bad = [
            r#"{"system":"any","root":"roaming","folder":"../x","checked_on":"2026-10-27"}"#,
            r#"{"system":"any","root":"roaming","folder":"x","checked_on":"27 Oct"}"#,
            r#"{"system":"any","root":"roaming","folder":"x","checked_on":"2026-13-01"}"#,
            r#"{"system":"any","root":"roaming","folder":"x","checked_on":"2026-10-27","relink":"http://x"}"#,
            r#"{"system":"any","root":"roaming","folder":"x","checked_on":"2026-10-27","export":{"kind":"open-page","url":"javascript:x"}}"#,
            r#"{"system":"any","root":"roaming","folder":"x","checked_on":"2026-10-27","export":{"kind":"tool","tool":"sh -c","args":[{"kind":"output-file"}]}}"#,
            r#"{"system":"any","root":"roaming","folder":"x","checked_on":"2026-10-27","export":{"kind":"tool","tool":"t","args":[{"kind":"switch","value":"; rm"},{"kind":"output-file"}]}}"#,
            r#"{"system":"any","root":"roaming","folder":"x","checked_on":"2026-10-27","export":{"kind":"tool","tool":"t","args":[]}}"#,
            r#"{"system":"any","root":"roaming","folder":"x","checked_on":"2026-10-27","exceptions":[{"members":"*","custody":"P","why":"looser"}]}"#,
            r#"{"system":"any","root":"roaming","folder":"x","checked_on":"2026-10-27","fixups":[{"kind":"ini-path","key":"a b","flag":null}]}"#,
            r#"{"system":"any","root":"roaming","folder":"x","checked_on":"2026-10-27","elsewhere":[{"system":"mac","root":"system-cache","folder":"x"}]}"#,
            r#"{"system":"any","root":"roaming","folder":"x","checked_on":"2026-10-27","export":{"kind":"run-shell","cmd":"x"}}"#,
        ];
        let c = load(&format!("{good},{}", bad.join(",")));
        assert_eq!(c.rows().len(), 1, "{:?}", c.report());
        assert_eq!(c.report().skipped.len(), bad.len());
        let row = &c.rows()[0];
        assert_eq!(
            row.tighten("tokens/a.json", Custody::Portable),
            (Custody::MachineBound, Some("sign-in tokens"))
        );
        assert_eq!(
            row.tighten("data/a.json", Custody::Portable),
            (Custody::Portable, None)
        );
        // Never looser: a stricter class stays.
        let strict = load(
            r#"{"system":"any","root":"roaming","folder":"x","checked_on":"2026-10-27","exceptions":[{"members":"*","custody":"E","why":"export"}]}"#,
        );
        assert_eq!(
            strict.rows()[0].tighten("a", Custody::MachineBound).0,
            Custody::MachineBound
        );
        assert_eq!(
            strict.rows()[0].tighten("a", Custody::Portable).0,
            Custody::NeedsExport
        );
        // Two rows naming the same folder contradict each other: both skipped.
        let twice = load(
            r#"{"system":"any","root":"roaming","folder":"X","checked_on":"2026-10-27","name":"a"},
               {"system":"any","root":"roaming","folder":"x","checked_on":"2026-10-27","name":"b"}"#,
        );
        assert!(twice.rows().is_empty());
    }

    #[test]
    fn a_row_matches_its_folder_on_its_systems_only() {
        let c = load(
            r#"{"system":"windows","root":"roaming","folder":"Big Vendor/App","checked_on":"2026-10-27"}"#,
        );
        let folder = AppDataFolder {
            root: RootKind::Roaming,
            root_path: std::path::PathBuf::from("r"),
            rel: "big vendor/app".into(),
            owner: pctwin_record::Owner::Shared,
            app: None,
            tie: None,
        };
        assert!(c.rows()[0].matches(System::Windows, &folder));
        assert!(!c.rows()[0].matches(System::Mac, &folder));
        let mut other = folder.clone();
        other.root = RootKind::Local;
        assert!(!c.rows()[0].matches(System::Windows, &other));
        assert!(Systems::Any.covers(System::Linux));
        assert!(!Systems::Linux.covers(System::Mac));
    }
}
