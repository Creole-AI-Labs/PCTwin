//! "Can I actually work?" checks (Task List 2.9; Product Spec R17; plan 2.7 section 4;
//! acceptance T27-005 to T27-007).
//!
//! The person picks the checks they care about ([`Choices::picked`]); nothing runs by default.
//! Each check only reads, a bounded amount, and never runs anything:
//!
//! - [`Goal::LinkedFiles`]: a project's linked files are there (each looked up, not opened);
//! - [`Goal::Font`]: a needed font is installed (each font file's own name table is read, not
//!   the file name guessed from);
//! - [`Goal::ArchiveOpens`]: a ZIP archive's index is whole (its end record and the start of its
//!   list of files; nothing is unpacked);
//! - [`Goal::DatabaseOpens`]: a SQLite database's header is sound (the first 100 bytes and the
//!   file's size; the database is never opened by SQLite, so nothing in it runs);
//! - [`Goal::PersonCheck`]: the person checks a task themselves and says so.
//!
//! A check that would have an effect outside the laptop or run something (send a message, print,
//! call an online service, start an app, run a macro, use the camera or microphone, connect to
//! a network service) is a [`Goal::External`]: it never borrows the move's approval, it needs its
//! own ([`EffectApproval`]), and even then PCTwin does not run it yet ("not built yet").
//!
//! Every result carries one of three labels ([`Label`]): machine-checked, person-confirmed or
//! still needs attention. A check of part of something says so ([`Coverage::Sample`]). What is
//! missing becomes a focused repair request to the part of PCTwin that owns it
//! ([`repair_requests`]); this part never edits a recipe or weakens a check to make it pass.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Component, Path, PathBuf};

use pctwin_record::ItemId;
use serde::{Deserialize, Serialize};

use crate::ledger::{Access, Coverage, Ledger, Role, Selection, Standing};

/// Most checks one person can pick at once.
pub const MAX_GOALS: usize = 200;
/// Most linked files one check looks up; the rest are counted as not checked.
pub const MAX_LINKS: usize = 10_000;
/// Most font files one check reads the names of.
pub const MAX_FONT_FILES: usize = 20_000;
/// Most bytes of a font file's name table read.
const MAX_NAME_TABLE: u32 = 1 << 20;
/// A ZIP archive's end record: 22 bytes and a comment of at most 65,535.
const ZIP_END: u64 = 22;
const ZIP_TAIL: u64 = ZIP_END + 65_535;

/// Something a check would do outside reading the laptop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExternalEffect {
    SendMessage,
    Print,
    CallOnlineService,
    StartApp,
    RunMacro,
    UseCameraOrMicrophone,
    ConnectToNetworkService,
}

/// One check the person can pick.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "goal")]
pub enum Goal {
    /// These files, relative to the project's folder `root`, are there.
    LinkedFiles { root: PathBuf, links: Vec<PathBuf> },
    /// A font of this family is installed in one of these folders.
    Font {
        family: String,
        folders: Vec<PathBuf>,
    },
    /// This ZIP archive's index is whole.
    ArchiveOpens { file: PathBuf },
    /// This SQLite database's header is sound.
    DatabaseOpens { file: PathBuf },
    /// The person does this task and says whether it worked.
    PersonCheck { task: String },
    /// A check with an effect outside reading the laptop.
    External {
        effect: ExternalEffect,
        task: String,
    },
}

/// One picked check: about which item, owned by which part of PCTwin (who repairs it).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chosen {
    pub item: ItemId,
    pub owner: Role,
    pub goal: Goal,
}

/// What the person picked, for one plan generation, and any separate approvals given.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Choices {
    generation: u64,
    chosen: Vec<Chosen>,
    effect_approved: Vec<bool>,
}

/// Why the person's choice was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ChoiceError {
    #[error("too many checks at once")]
    TooMany,
    #[error("no such check")]
    NoSuchCheck,
    #[error("the answer is for another version of the plan")]
    StalePlan,
    #[error("that check has no outside effect to approve")]
    NothingToApprove,
    #[error("that answer is for another kind of check")]
    WrongKind,
}

/// The person's separate approval of one outside effect of one picked check, for one plan
/// generation. It is made only from the person's own answer to that check; a move's approval is
/// another type and cannot stand in for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectApproval {
    index: usize,
    generation: u64,
    goal: [u8; 32],
}

fn goal_digest(c: &Chosen) -> [u8; 32] {
    let mut h = blake3::Hasher::new();
    h.update(b"pctwin/v1/readiness-goal\0");
    h.update(&serde_json::to_vec(c).expect("goals always serialise"));
    *h.finalize().as_bytes()
}

impl EffectApproval {
    /// The person approved the outside effect of check `index` of `choices`, shown for
    /// `generation`.
    pub fn from_person(
        choices: &Choices,
        index: usize,
        generation: u64,
    ) -> Result<Self, ChoiceError> {
        let c = choices.chosen.get(index).ok_or(ChoiceError::NoSuchCheck)?;
        if !matches!(c.goal, Goal::External { .. }) {
            return Err(ChoiceError::NothingToApprove);
        }
        Ok(Self {
            index,
            generation,
            goal: goal_digest(c),
        })
    }
}

impl Choices {
    /// The checks the person picked. Nothing is checked that is not picked.
    pub fn picked(generation: u64, chosen: Vec<Chosen>) -> Result<Self, ChoiceError> {
        if chosen.len() > MAX_GOALS {
            return Err(ChoiceError::TooMany);
        }
        let effect_approved = vec![false; chosen.len()];
        Ok(Self {
            generation,
            chosen,
            effect_approved,
        })
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn chosen(&self) -> &[Chosen] {
        &self.chosen
    }

    /// Records the person's separate approval of one outside effect, if it is for this very check
    /// and this plan generation.
    pub fn approve_effect(&mut self, a: EffectApproval) -> Result<(), ChoiceError> {
        let c = self.chosen.get(a.index).ok_or(ChoiceError::NoSuchCheck)?;
        if a.generation != self.generation {
            return Err(ChoiceError::StalePlan);
        }
        if goal_digest(c) != a.goal {
            return Err(ChoiceError::WrongKind);
        }
        self.effect_approved[a.index] = true;
        Ok(())
    }
}

/// The three labels a result carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Label {
    MachineChecked,
    PersonConfirmed,
    StillNeedsAttention,
}

/// What a check found missing or wrong.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "missing")]
pub enum Missing {
    /// A linked file is not there.
    LinkedFile { name: String },
    /// No installed font has this family name.
    Font { family: String },
    /// The file is not whole, or not what it should be.
    Damaged { what: Damage },
    /// The file is not there at all.
    NotThere,
    /// The person said the task did not work.
    PersonSaidNo,
}

/// How a file was found not whole.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Damage {
    /// No ZIP end record: cut short, or not a ZIP archive.
    NoArchiveEnd,
    /// The archive's index points outside the file.
    ArchiveIndexOutside,
    /// The archive is in several parts; only one is here.
    ArchiveInParts,
    /// Not a SQLite database.
    NotADatabase,
    /// The database's header is not sound, or its size does not fit its pages.
    DatabaseHeader,
}

/// Why a check did not run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NotRun {
    /// It waits for the person's answer.
    WaitingForPerson,
    /// It has an effect outside reading the laptop and needs its own approval.
    NeedsItsOwnApproval,
    /// Approved, but PCTwin cannot do this check yet.
    NotBuiltYet,
    /// A linked file outside the project's folder (an absolute path, `..`): not looked up.
    OutsideTheProject,
    /// The file could not be read (no permission, say).
    CouldNotRead,
}

/// How one check ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "outcome")]
pub enum Outcome {
    MachineChecked {
        coverage: Coverage,
    },
    PersonConfirmed {
        coverage: Coverage,
    },
    NeedsAttention {
        missing: Vec<Missing>,
        coverage: Coverage,
    },
    NotRun {
        why: NotRun,
    },
}

impl Outcome {
    /// The label it carries (a check that did not run still needs attention).
    pub fn label(&self) -> Label {
        match self {
            Outcome::MachineChecked { .. } => Label::MachineChecked,
            Outcome::PersonConfirmed { .. } => Label::PersonConfirmed,
            Outcome::NeedsAttention { .. } | Outcome::NotRun { .. } => Label::StillNeedsAttention,
        }
    }

    fn coverage(&self) -> Option<Coverage> {
        match self {
            Outcome::MachineChecked { coverage }
            | Outcome::PersonConfirmed { coverage }
            | Outcome::NeedsAttention { coverage, .. } => Some(*coverage),
            Outcome::NotRun { .. } => None,
        }
    }
}

/// One check's result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckResult {
    /// The check's place in the person's choices.
    pub index: usize,
    pub item: ItemId,
    pub generation: u64,
    pub outcome: Outcome,
}

/// Runs every picked check, reading only.
pub fn run(choices: &Choices) -> Vec<CheckResult> {
    choices
        .chosen
        .iter()
        .enumerate()
        .map(|(index, c)| CheckResult {
            index,
            item: c.item,
            generation: choices.generation,
            outcome: run_one(&c.goal, choices.effect_approved[index]),
        })
        .collect()
}

fn run_one(goal: &Goal, effect_approved: bool) -> Outcome {
    match goal {
        Goal::LinkedFiles { root, links } => linked_files(root, links),
        Goal::Font { family, folders } => font(family, folders),
        Goal::ArchiveOpens { file } => archive(file),
        Goal::DatabaseOpens { file } => database(file),
        Goal::PersonCheck { .. } => Outcome::NotRun {
            why: NotRun::WaitingForPerson,
        },
        Goal::External { .. } if effect_approved => Outcome::NotRun {
            why: NotRun::NotBuiltYet,
        },
        Goal::External { .. } => Outcome::NotRun {
            why: NotRun::NeedsItsOwnApproval,
        },
    }
}

/// The person's answer to check `index`: it worked, for `coverage` of it, or it did not.
pub fn person_answer(
    choices: &Choices,
    index: usize,
    generation: u64,
    worked: bool,
    coverage: Coverage,
) -> Result<CheckResult, ChoiceError> {
    let c = choices.chosen.get(index).ok_or(ChoiceError::NoSuchCheck)?;
    if generation != choices.generation {
        return Err(ChoiceError::StalePlan);
    }
    if !matches!(c.goal, Goal::PersonCheck { .. }) {
        return Err(ChoiceError::WrongKind);
    }
    let outcome = if worked {
        Outcome::PersonConfirmed { coverage }
    } else {
        Outcome::NeedsAttention {
            missing: vec![Missing::PersonSaidNo],
            coverage,
        }
    };
    Ok(CheckResult {
        index,
        item: c.item,
        generation,
        outcome,
    })
}

/// Whether `link` stays inside its project's folder: relative, with no `..`.
fn inside(link: &Path) -> bool {
    link.components()
        .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
        && link.components().next().is_some()
}

fn linked_files(root: &Path, links: &[PathBuf]) -> Outcome {
    if links.iter().any(|l| !inside(l)) {
        return Outcome::NotRun {
            why: NotRun::OutsideTheProject,
        };
    }
    let checked = links.len().min(MAX_LINKS);
    let coverage = if links.is_empty() {
        Coverage::Whole
    } else {
        Coverage::sample(checked as u64, links.len() as u64).expect("at least one, at most all")
    };
    let missing: Vec<Missing> = links[..checked]
        .iter()
        .filter(|l| std::fs::symlink_metadata(root.join(l)).is_err())
        .map(|l| Missing::LinkedFile {
            name: l.to_string_lossy().into_owned(),
        })
        .collect();
    if missing.is_empty() {
        Outcome::MachineChecked { coverage }
    } else {
        Outcome::NeedsAttention { missing, coverage }
    }
}

/// A family name compared without case, spaces, dashes or underscores.
fn plain_name(name: &str) -> String {
    name.chars()
        .filter(|c| !matches!(c, ' ' | '-' | '_'))
        .flat_map(char::to_lowercase)
        .collect()
}

fn font(family: &str, folders: &[PathBuf]) -> Outcome {
    let want = plain_name(family);
    // Every font file in the folders is listed (names only); the names inside the first
    // MAX_FONT_FILES are read. A family not found among only some of them is said so.
    let mut fonts: Vec<PathBuf> = Vec::new();
    for folder in folders {
        let Ok(dir) = std::fs::read_dir(folder) else {
            continue;
        };
        fonts.extend(dir.flatten().map(|e| e.path()).filter(|p| {
            p.extension().and_then(|e| e.to_str()).is_some_and(|e| {
                matches!(
                    e.to_ascii_lowercase().as_str(),
                    "ttf" | "otf" | "ttc" | "otc"
                )
            })
        }));
    }
    let checked = fonts.len().min(MAX_FONT_FILES);
    if fonts[..checked]
        .iter()
        .any(|p| font_families(p).iter().any(|f| plain_name(f) == want))
    {
        return Outcome::MachineChecked {
            coverage: Coverage::Whole,
        };
    }
    let coverage = Coverage::sample(checked as u64, fonts.len() as u64).unwrap_or(Coverage::Whole);
    Outcome::NeedsAttention {
        missing: vec![Missing::Font {
            family: family.to_string(),
        }],
        coverage,
    }
}

fn read_at(f: &mut File, at: u64, buf: &mut [u8]) -> std::io::Result<()> {
    f.seek(SeekFrom::Start(at))?;
    f.read_exact(buf)
}

fn be16(b: &[u8], at: usize) -> u16 {
    u16::from_be_bytes([b[at], b[at + 1]])
}

fn be32(b: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

/// The family names (name IDs 1 and 16) of the font file at `path`, read from its own name table:
/// every font of a collection. Unreadable or unknown files give none.
pub fn font_families(path: &Path) -> Vec<String> {
    let Ok(mut f) = File::open(path) else {
        return Vec::new();
    };
    let mut head = [0u8; 12];
    if read_at(&mut f, 0, &mut head).is_err() {
        return Vec::new();
    }
    let starts: Vec<u64> = if &head[..4] == b"ttcf" {
        let n = be32(&head, 8).min(64);
        let mut offs = vec![0u8; n as usize * 4];
        if read_at(&mut f, 12, &mut offs).is_err() {
            return Vec::new();
        }
        (0..n as usize)
            .map(|i| u64::from(be32(&offs, i * 4)))
            .collect()
    } else {
        vec![0]
    };
    let mut out = Vec::new();
    for start in starts {
        out.extend(names_of_font(&mut f, start).unwrap_or_default());
    }
    out
}

fn names_of_font(f: &mut File, start: u64) -> std::io::Result<Vec<String>> {
    let mut head = [0u8; 12];
    read_at(f, start, &mut head)?;
    let version = be32(&head, 0);
    if !matches!(version, 0x0001_0000 | 0x4F54_544F | 0x7472_7565) {
        return Ok(Vec::new());
    }
    let tables = usize::from(be16(&head, 4));
    let mut recs = vec![0u8; tables * 16];
    read_at(f, start + 12, &mut recs)?;
    let Some(rec) = recs.as_chunks::<16>().0.iter().find(|r| &r[..4] == b"name") else {
        return Ok(Vec::new());
    };
    let (offset, length) = (be32(rec, 8), be32(rec, 12));
    if !(6..=MAX_NAME_TABLE).contains(&length) {
        return Ok(Vec::new());
    }
    let mut table = vec![0u8; length as usize];
    read_at(f, u64::from(offset), &mut table)?;
    let count = usize::from(be16(&table, 2));
    let strings = usize::from(be16(&table, 4));
    let mut out = Vec::new();
    for i in 0..count {
        let at = 6 + i * 12;
        let Some(r) = table.get(at..at + 12) else {
            break;
        };
        let (platform, encoding, name_id) = (be16(r, 0), be16(r, 2), be16(r, 6));
        let (len, off) = (usize::from(be16(r, 8)), usize::from(be16(r, 10)));
        if !matches!(name_id, 1 | 16) {
            continue;
        }
        let Some(bytes) = table.get(strings + off..strings + off + len) else {
            continue;
        };
        let text = match (platform, encoding) {
            // Unicode, and Windows' Unicode encodings: UTF-16 big-endian.
            (0, _) | (3, 0 | 1 | 10) => {
                let units: Vec<u16> = bytes
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|c| u16::from_be_bytes(*c))
                    .collect();
                String::from_utf16_lossy(&units)
            }
            // Mac Roman: family names are plain letters in practice; others are left out.
            (1, 0) if bytes.is_ascii() => String::from_utf8_lossy(bytes).into_owned(),
            _ => continue,
        };
        if !text.is_empty() {
            out.push(text);
        }
    }
    Ok(out)
}

fn damaged(what: Damage) -> Outcome {
    Outcome::NeedsAttention {
        missing: vec![Missing::Damaged { what }],
        coverage: Coverage::Whole,
    }
}

fn not_there_or_unreadable(e: &std::io::Error) -> Outcome {
    if e.kind() == std::io::ErrorKind::NotFound {
        Outcome::NeedsAttention {
            missing: vec![Missing::NotThere],
            coverage: Coverage::Whole,
        }
    } else {
        Outcome::NotRun {
            why: NotRun::CouldNotRead,
        }
    }
}

/// Checks a ZIP archive's index without unpacking anything: the end record must close the file
/// exactly (its comment length reaches the last byte), name one part only, and point to a list
/// of files that lies inside the file and starts with that list's own signature.
fn archive(path: &Path) -> Outcome {
    let mut f = match File::open(path) {
        Ok(f) => f,
        Err(e) => return not_there_or_unreadable(&e),
    };
    let len = match f.metadata() {
        Ok(m) => m.len(),
        Err(e) => return not_there_or_unreadable(&e),
    };
    if len < ZIP_END {
        return damaged(Damage::NoArchiveEnd);
    }
    let tail_len = len.min(ZIP_TAIL);
    let mut tail = vec![0u8; tail_len as usize];
    if read_at(&mut f, len - tail_len, &mut tail).is_err() {
        return Outcome::NotRun {
            why: NotRun::CouldNotRead,
        };
    }
    let le16 = |b: &[u8], at: usize| u16::from_le_bytes([b[at], b[at + 1]]);
    let le32 = |b: &[u8], at: usize| u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]]);
    // The last end record whose comment reaches exactly the end of the file.
    let found = (0..=tail.len() - ZIP_END as usize).rev().find(|&p| {
        tail[p..p + 4] == [0x50, 0x4b, 0x05, 0x06]
            && p + ZIP_END as usize + usize::from(le16(&tail, p + 20)) == tail.len()
    });
    let Some(p) = found else {
        return damaged(Damage::NoArchiveEnd);
    };
    let end_at = len - tail_len + p as u64;
    let (disk, cd_disk) = (le16(&tail, p + 4), le16(&tail, p + 6));
    let (on_disk, total) = (le16(&tail, p + 8), le16(&tail, p + 10));
    let (cd_size, cd_offset) = (le32(&tail, p + 12), le32(&tail, p + 16));
    if cd_size == u32::MAX || cd_offset == u32::MAX || total == u16::MAX {
        // ZIP64: its locator must sit just before the end record.
        let ok = p >= 20 && tail[p - 20..p - 16] == [0x50, 0x4b, 0x06, 0x07];
        return if ok {
            Outcome::MachineChecked {
                coverage: Coverage::Whole,
            }
        } else {
            damaged(Damage::NoArchiveEnd)
        };
    }
    if disk != 0 || cd_disk != 0 || on_disk != total {
        return damaged(Damage::ArchiveInParts);
    }
    if u64::from(cd_offset) + u64::from(cd_size) > end_at {
        return damaged(Damage::ArchiveIndexOutside);
    }
    if total > 0 {
        let mut sig = [0u8; 4];
        if read_at(&mut f, u64::from(cd_offset), &mut sig).is_err()
            || sig != [0x50, 0x4b, 0x01, 0x02]
        {
            return damaged(Damage::ArchiveIndexOutside);
        }
    }
    Outcome::MachineChecked {
        coverage: Coverage::Whole,
    }
}

/// Checks a SQLite database's header (sqlite.org/fileformat.html) and that the file is a whole
/// number of its pages. The database is never opened by SQLite.
fn database(path: &Path) -> Outcome {
    let mut f = match File::open(path) {
        Ok(f) => f,
        Err(e) => return not_there_or_unreadable(&e),
    };
    let len = match f.metadata() {
        Ok(m) => m.len(),
        Err(e) => return not_there_or_unreadable(&e),
    };
    let mut h = [0u8; 100];
    if len < 100 || read_at(&mut f, 0, &mut h).is_err() || &h[..16] != b"SQLite format 3\0" {
        return damaged(Damage::NotADatabase);
    }
    let page = match be16(&h, 16) {
        1 => 65_536u64,
        p if p.is_power_of_two() && (512..=32_768).contains(&p) => u64::from(p),
        _ => return damaged(Damage::DatabaseHeader),
    };
    let versions_ok = matches!(h[18], 1 | 2) && matches!(h[19], 1 | 2);
    let fractions_ok = h[21] == 64 && h[22] == 32 && h[23] == 32;
    if !versions_ok || !fractions_ok || len % page != 0 {
        return damaged(Damage::DatabaseHeader);
    }
    // The page count in the header is in force only when its "valid for" number matches the
    // change counter; then the file must hold at least that many pages.
    let pages = u64::from(be32(&h, 28));
    if be32(&h, 92) == be32(&h, 24) && pages * page > len {
        return damaged(Damage::DatabaseHeader);
    }
    Outcome::MachineChecked {
        coverage: Coverage::Whole,
    }
}

/// A focused repair request: what is missing for an item, sent to the part of PCTwin that owns it
/// (T27-007).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairRequest {
    pub item: ItemId,
    pub to: Role,
    pub need: Missing,
}

/// The repair requests for what the checks found missing: a linked file or a font goes to app
/// data (2.5, which brings project dependencies and fonts); a damaged file to the part that
/// brought it; a task the person says failed goes back to the item's owner too.
pub fn repair_requests(choices: &Choices, results: &[CheckResult]) -> Vec<RepairRequest> {
    let mut out = Vec::new();
    for r in results {
        let Some(c) = choices.chosen.get(r.index) else {
            continue;
        };
        if r.generation != choices.generation || r.item != c.item {
            continue;
        }
        if let Outcome::NeedsAttention { missing, .. } = &r.outcome {
            for m in missing {
                let to = match m {
                    Missing::LinkedFile { .. } | Missing::Font { .. } => Role::Data,
                    Missing::Damaged { .. } | Missing::NotThere | Missing::PersonSaidNo => c.owner,
                };
                out.push(RepairRequest {
                    item: c.item,
                    to,
                    need: m.clone(),
                });
            }
        }
    }
    out
}

/// What can be said of the move as a whole (plan 2.7 section 10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Summary {
    /// Everything selected arrived, and every check the person picked passed.
    ReadyForSelectedWork,
    /// Everything selected arrived and was checked as copied; no work was checked.
    DataPreserved,
    /// Some things still need doing.
    FinishedWithTasks,
}

impl Summary {
    pub fn words(&self) -> &'static str {
        match self {
            Summary::ReadyForSelectedWork => {
                "Ready for the work you checked. Only the checks you picked were done."
            }
            Summary::DataPreserved => {
                "Everything you picked is on the new laptop and was checked as it arrived. \
                 PCTwin has not checked that your work opens: pick a check to try it."
            }
            Summary::FinishedWithTasks => {
                "The move finished, with some things left to do. They are listed below."
            }
        }
    }
}

/// The whole-move summary from the ledger and the checks of this generation (T27-005): a launched
/// app or a copied file never makes the selected work "ready" while a picked check failed, did
/// not run, or a key is missing.
pub fn summarize(ledger: &Ledger, results: &[CheckResult]) -> Summary {
    let mut all_done = true;
    for (id, line) in ledger.lines() {
        if line.selection != Selection::Selected {
            continue;
        }
        let done = matches!(ledger.standing(*id), Some(Standing::Done(_)));
        let key_ok = ledger.access(*id) == Some(Access::Usable);
        if !done || !key_ok {
            all_done = false;
        }
    }
    let current: Vec<&CheckResult> = results
        .iter()
        .filter(|r| r.generation == ledger.generation())
        .collect();
    if !all_done
        || current
            .iter()
            .any(|r| r.outcome.label() == Label::StillNeedsAttention)
    {
        return Summary::FinishedWithTasks;
    }
    if current.is_empty() {
        Summary::DataPreserved
    } else {
        Summary::ReadyForSelectedWork
    }
}

/// Whether any of these results checked only a sample (the summary then says so).
pub fn any_sample(results: &[CheckResult]) -> bool {
    results
        .iter()
        .any(|r| r.outcome.coverage().is_some_and(|c| !c.is_whole()))
}
