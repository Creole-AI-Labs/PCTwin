//! The rules file (`rules/places.txt`): places whose kind is known from where they are. A hit
//! decides before anyone's access is looked at, and the sweep only counts what is inside.

use serde::{Deserialize, Serialize};

/// What a place is, from the rules file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    /// Part of the operating system.
    System,
    /// Installed programs and what they keep for everyone.
    App,
    /// What one program keeps for one person.
    AppData,
    /// Caches, temporary files and bins.
    Junk,
    /// Can be made again (downloaded packages, build output).
    Rebuildable,
    /// The person's own, counted and moved as one item (a project's history).
    KeepWhole,
}

/// Where a row applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Where {
    /// Directly under the root of any drive.
    Root,
    /// Directly under the root of the system drive.
    SystemRoot,
    /// Directly under a home folder.
    Home,
    /// A path below a home folder.
    Under,
    /// A folder with this name anywhere.
    Any,
}

/// The systems a row is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum System {
    Windows,
    Mac,
    Linux,
    Any,
}

impl System {
    /// The system this build runs on.
    pub fn this() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::Mac
        } else {
            Self::Linux
        }
    }

    fn ignores_case(self) -> bool {
        matches!(self, Self::Windows | Self::Mac)
    }
}

/// One row of the rules file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub system: System,
    pub place: Where,
    /// One name, or for [`Where::Under`] names separated by `/`.
    pub name: String,
    pub kind: Kind,
    /// The line in the rules file, so every placement can say which rule placed it.
    pub line: usize,
}

impl Row {
    /// How a placement names this rule: `places.txt:12 node_modules`.
    pub fn label(&self) -> String {
        format!("places.txt:{} {}", self.line, self.name)
    }
}

/// The rules file, read once.
#[derive(Debug, Clone)]
pub struct Rules {
    rows: Vec<Row>,
    system: System,
}

const PLACES: &str = include_str!("../rules/places.txt");

/// Why the rules file could not be read.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("rules file line {line}: {why}")]
pub struct RulesError {
    pub line: usize,
    pub why: String,
}

impl Rules {
    /// The rules shipped with PCTwin, for the system this build runs on.
    pub fn shipped() -> Self {
        // The shipped file is checked by a test; a broken one must never be half-applied.
        Self::parse(PLACES, System::this()).expect("the shipped rules file reads")
    }

    /// Reads a rules file for `system`.
    pub fn parse(text: &str, system: System) -> Result<Self, RulesError> {
        let mut rows = Vec::new();
        for (i, line) in text.lines().enumerate() {
            let line_no = i + 1;
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let bad = |why: &str| RulesError {
                line: line_no,
                why: why.to_string(),
            };
            let fields: Vec<&str> = trimmed.split('|').map(str::trim).collect();
            let [sys, place, name, kind] = fields[..] else {
                return Err(bad("a row has four parts separated by |"));
            };
            let row_system = match sys {
                "windows" => System::Windows,
                "mac" => System::Mac,
                "linux" => System::Linux,
                "any" => System::Any,
                _ => return Err(bad("unknown system")),
            };
            let place = match place {
                "root" => Where::Root,
                "sysroot" => Where::SystemRoot,
                "home" => Where::Home,
                "under" => Where::Under,
                "any" => Where::Any,
                _ => return Err(bad("unknown place")),
            };
            let kind = match kind {
                "system" => Kind::System,
                "app" => Kind::App,
                "app-data" => Kind::AppData,
                "junk" => Kind::Junk,
                "rebuildable" => Kind::Rebuildable,
                "keep-whole" => Kind::KeepWhole,
                _ => return Err(bad("unknown kind")),
            };
            if name.is_empty() || (place != Where::Under && name.contains('/')) {
                return Err(bad("a name is empty or holds a /"));
            }
            if row_system == system || row_system == System::Any {
                rows.push(Row {
                    system: row_system,
                    place,
                    name: name.to_string(),
                    kind,
                    line: line_no,
                });
            }
        }
        Ok(Self { rows, system })
    }

    fn same(&self, a: &str, b: &str) -> bool {
        if self.system.ignores_case() {
            a.to_lowercase() == b.to_lowercase()
        } else {
            a == b
        }
    }

    /// The row for a folder or file named `name` directly under a drive's root.
    pub fn at_root(&self, name: &str, system_drive: bool) -> Option<&Row> {
        self.rows.iter().find(|r| {
            (r.place == Where::Root || (system_drive && r.place == Where::SystemRoot))
                && self.same(&r.name, name)
        })
    }

    /// The row for something at `parts` below a home folder (`["AppData", "Local", "Temp"]`).
    pub fn in_home(&self, parts: &[&str]) -> Option<&Row> {
        self.rows.iter().find(|r| match r.place {
            Where::Home => parts.len() == 1 && self.same(&r.name, parts[0]),
            Where::Under => {
                let want: Vec<&str> = r.name.split('/').collect();
                want.len() == parts.len() && want.iter().zip(parts).all(|(w, p)| self.same(w, p))
            }
            _ => false,
        })
    }

    /// The row for a folder named `name` wherever it is.
    pub fn anywhere(&self, name: &str) -> Option<&Row> {
        self.rows
            .iter()
            .find(|r| r.place == Where::Any && self.same(&r.name, name))
    }

    pub fn rows(&self) -> &[Row] {
        &self.rows
    }
}
