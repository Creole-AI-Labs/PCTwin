//! The offline recovery kit (Engineering Plan A02; Task List 2.9 "Recovery when PCTwin itself
//! breaks"; acceptance T27-025 for what it says).
//!
//! The move record is kept and can be exported without any account: a plain JSON file of the move
//! (what was picked, the result and evidence of every item, the before-wipe list), and a plain
//! text "read me" that says exactly what the kit holds and what it does not. This version holds no
//! file data, no setup recipe and no recovery tool, and it says so: instructions alone cannot
//! bring back missing files.

use std::path::{Path, PathBuf};

use pctwin_record::Record;
use serde::{Deserialize, Serialize};

use crate::goodbye::Checklist;
use crate::ledger::Ledger;
use crate::store;

/// The kit's format.
pub const KIT_FORMAT: u32 = 1;
/// The kit's file name.
pub const KIT_FILE: &str = "PCTwin recovery kit.json";
/// The read-me's file name.
pub const README_FILE: &str = "PCTwin recovery kit - read me.txt";

/// What a kit holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contents {
    pub move_record: bool,
    pub results: bool,
    pub before_wipe_list: bool,
    /// Copies of the person's files (not in this version).
    pub file_data: bool,
    /// The steps to set apps and settings up again (not in this version).
    pub setup_recipe: bool,
    /// A tool that restores without PCTwin (not in this version).
    pub recovery_tool: bool,
}

/// The recovery kit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kit {
    format: u32,
    pub made_at: u64,
    pub contents: Contents,
    pub record: Record,
    pub ledger: Ledger,
    pub checklist: Option<Checklist>,
}

/// Why a kit could not be read.
#[derive(Debug, thiserror::Error)]
pub enum KitError {
    #[error("this kit was made by a newer PCTwin (format {found}); use that version to read it")]
    NewerFormat { found: u32 },
    #[error("this kit is damaged and cannot be read")]
    Damaged,
    #[error("this kit's parts are from different moves")]
    Mixed,
    #[error("this kit says it holds something it does not")]
    Partial,
    #[error("the kit could not be read: {0}")]
    Io(#[from] std::io::Error),
}

/// A date, in UTC, from seconds since 1970 (days to a civil date: Howard Hinnant's method).
pub fn date(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

impl Kit {
    /// A kit of this move: its record, its ledger and, when made, its before-wipe list.
    pub fn new(record: Record, ledger: Ledger, checklist: Option<Checklist>, made_at: u64) -> Self {
        let contents = Contents {
            move_record: true,
            results: true,
            before_wipe_list: checklist.is_some(),
            file_data: false,
            setup_recipe: false,
            recovery_tool: false,
        };
        Self {
            format: KIT_FORMAT,
            made_at,
            contents,
            record,
            ledger,
            checklist,
        }
    }

    /// The read-me: exactly what the kit holds, and what it does not.
    pub fn readme(&self) -> String {
        let c = &self.contents;
        let mut has =
            vec!["- your move record: what was picked to move, and where each thing went;"];
        if c.results {
            has.push("- the result of every item, and how each result was checked;");
        }
        if c.before_wipe_list {
            has.push("- the list of what is still only on the old laptop.");
        }
        let mut not = Vec::new();
        if !c.file_data {
            not.push(
                "- any of your files. These instructions alone cannot bring back missing files: \
                 keep the old laptop, or your own backup, until you no longer need them.",
            );
        }
        if !c.setup_recipe {
            not.push("- the steps to set your apps and settings up again.");
        }
        if !c.recovery_tool {
            not.push("- a tool to restore anything: read the record with any text editor.");
        }
        format!(
            "PCTwin recovery kit, made on {}.\n\nThis kit holds:\n{}\n\nThis kit does not hold:\n{}\n\n\
             You need no account to read it. \"{KIT_FILE}\" is a plain text file (JSON) that any \
             text editor opens. Keep it somewhere safe: it names your files.\n",
            date(self.made_at),
            has.join("\n"),
            not.join("\n"),
        )
    }

    /// Exports the kit into `folder` (one the person picked): the kit and its read-me, each a new
    /// file, never over one already there. No account, no network.
    pub fn export(&self, folder: &Path) -> std::io::Result<(PathBuf, PathBuf)> {
        let json = serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?;
        let kit = store::save_new(folder, KIT_FILE, &json)?;
        let readme = store::save_new(folder, README_FILE, self.readme().as_bytes())?;
        Ok((kit, readme))
    }

    /// Reads a kit back, checking its format first and that its parts belong together.
    pub fn read(path: &Path) -> Result<Self, KitError> {
        let value: serde_json::Value = store::load(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                KitError::Io(e)
            } else {
                KitError::Damaged
            }
        })?;
        let found = value
            .get("format")
            .and_then(serde_json::Value::as_u64)
            .ok_or(KitError::Damaged)?;
        if found > u64::from(KIT_FORMAT) {
            return Err(KitError::NewerFormat {
                found: u32::try_from(found).unwrap_or(u32::MAX),
            });
        }
        let kit: Kit = serde_json::from_value(value).map_err(|_| KitError::Damaged)?;
        kit.record.validate().map_err(|_| KitError::Damaged)?;
        if kit.ledger.id().source != kit.record.source_laptop
            || kit
                .checklist
                .as_ref()
                .is_some_and(|c| c.move_id != kit.ledger.id())
        {
            return Err(KitError::Mixed);
        }
        let c = kit.contents;
        if !c.move_record
            || c.before_wipe_list != kit.checklist.is_some()
            || c.file_data
            || c.setup_recipe
            || c.recovery_tool
        {
            return Err(KitError::Partial);
        }
        Ok(kit)
    }
}
