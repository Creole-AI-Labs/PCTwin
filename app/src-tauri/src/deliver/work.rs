//! "Can I actually work?" checks on the screens (Task List 2.9; Product Spec R17).
//!
//! The checks themselves are `pctwin_readiness::checks`: bounded, read only, never running
//! anything. A file the person picks is told by its content first (an archive, a database), then
//! by the `project-files` catalogue's generic readers (`pctwin_readiness::projects`): its linked
//! files and every font it names become checks of their own. A file PCTwin cannot check is kept
//! in the list as "not supported", never passed as fine. The list is kept in PCTwin's app data
//! folder, on this laptop only.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use pctwin_catalogue::Catalogue;
use pctwin_readiness::checks::{
    self, ChoiceError, Choices, Chosen, Goal, Missing, NotRun, Outcome,
};
use pctwin_readiness::ledger::{Coverage, Role};
use pctwin_readiness::projects::{self, Found};
use pctwin_record::ItemId;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime};

/// What the person adds a check for.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum WorkPick {
    /// A file they work with, picked in the system's own file picker: a project whose linked
    /// files and fonts are looked up, an archive whose index is read, or a database whose header
    /// is read (told by its content, never its name).
    File {},
    /// A font they need, by its family name.
    Font { family: String },
    /// A task they do themselves and say whether it worked.
    Task { task: String },
}

pctwin_types::listed_enum! {
    /// What a check looks at.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum WorkWhat {
        /// A project's linked files.
        LinkedFiles,
        /// A font.
        Font,
        /// An archive opens.
        Archive,
        /// A database opens.
        Database,
        /// A task the person does.
        Task,
    }
}

pctwin_types::listed_enum! {
    /// The three labels every result carries.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum WorkLabel {
        MachineChecked,
        PersonConfirmed,
        StillNeedsAttention,
    }
}

pctwin_types::listed_enum! {
    /// Where one check got to.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum WorkState {
        /// Added, not run yet.
        NotRunYet,
        /// Everything it looked at is there and whole.
        AllThere,
        /// Something it needs is missing (named in `missing`).
        Missing,
        /// The file is not whole, or not what it should be.
        Damaged,
        /// The file is not there at all.
        NotThere,
        /// Waits for the person to try it and say.
        WaitingForYou,
        /// The person said it did not work.
        YouSaidNo,
        /// It links to files outside the project's folder: not looked up.
        OutsideProject,
        /// The file could not be read (no permission, say).
        CouldNotRead,
        /// PCTwin cannot check this kind of file yet: found, not yet supported.
        NotSupported,
        /// What it looked at is there, but not everything was looked at (a file longer than
        /// PCTwin reads, more linked files or fonts than one check takes): still needs attention.
        PartlyChecked,
    }
}

/// One check and its latest result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkCheck {
    /// Its place in the list (for answering or removing it).
    pub index: u32,
    pub what: WorkWhat,
    /// The file, font or task, shown exactly as it is.
    pub name: String,
    pub label: WorkLabel,
    pub state: WorkState,
    /// What is missing (file or font names, shown exactly as they are), bounded.
    pub missing: Vec<String>,
    pub missing_more: u64,
    /// Kinds of file found and not checked (a font file PCTwin cannot read the names of).
    pub not_checked: Vec<String>,
    /// How much was looked at: `checked` of `of` (equal when all of it).
    pub checked: u64,
    pub of: u64,
}

/// The person's checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkChecks {
    /// The list's version: an answer or removal for another version is refused.
    pub generation: u64,
    pub checks: Vec<WorkCheck>,
    pub more: u64,
}

pctwin_types::listed_enum! {
    /// Why a check could not be added, answered or removed.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum WorkProblem {
        /// The person chose no file in the picker.
        NotChosen,
        /// Too many checks at once.
        TooMany,
        /// No such check.
        NoSuchCheck,
        /// The list changed since the screens saw it.
        Stale,
        /// A font or task name that is empty or too long.
        BadName,
        /// The answer is for another kind of check.
        WrongKind,
        Failed,
    }
}

/* ---------- The checks (real) ---------- */

/// The most checks one person keeps (the readiness checks' own limit).
pub const MAX_CHECKS: usize = pctwin_readiness::checks::MAX_GOALS;
/// The longest font or task name, in characters.
pub const MAX_NAME_CHARS: usize = 200;
/// The most missing names sent for one check (the rest are counted).
const MAX_MISSING: usize = 50;

/// One check the person added.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Entry {
    what: WorkWhat,
    name: String,
    /// What the readiness checks run (none: a file PCTwin cannot check, or could not read).
    goal: Option<Goal>,
    /// Set when no goal could be made: why.
    state: Option<WorkState>,
    /// Linked files outside the project's folder: counted, never looked up.
    outside: u64,
    /// The file was longer than PCTwin reads: only its start was looked at.
    #[serde(default)]
    partly_read: bool,
    /// Linked files the file names past the most one check takes: counted, never looked up.
    #[serde(default)]
    links_left: u64,
    /// Fonts the file names that got no check of their own (past the most one file lists, or
    /// past the most checks one person keeps).
    #[serde(default)]
    fonts_left: u64,
    /// The latest result.
    outcome: Option<Outcome>,
}

impl Entry {
    fn new(what: WorkWhat, name: String, goal: Option<Goal>, state: Option<WorkState>) -> Self {
        Self {
            what,
            name,
            goal,
            state,
            outside: 0,
            partly_read: false,
            links_left: 0,
            fonts_left: 0,
            outcome: None,
        }
    }

    /// Something it should have looked at was not looked at.
    fn partly(&self) -> bool {
        self.partly_read || self.links_left > 0 || self.fonts_left > 0
    }
}

/// The person's checks, in the order added.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Work {
    generation: u64,
    entries: Vec<Entry>,
}

fn item_of(name: &str, index: usize) -> ItemId {
    let h = blake3::Hasher::new_derive_key("PCTwin work check item 2026-10-29 v1")
        .update(name.as_bytes())
        .update(&(index as u64).to_le_bytes())
        .finalize();
    let hex = h.to_hex();
    ItemId::from_hex(&hex[..32]).expect("32 hexadecimal digits are an item ID")
}

fn name_ok(s: &str) -> Option<String> {
    let s = s.trim();
    (!s.is_empty() && s.chars().count() <= MAX_NAME_CHARS && !s.chars().any(char::is_control))
        .then(|| s.to_string())
}

impl Work {
    fn bump(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    fn room(&self, n: usize) -> Result<(), WorkProblem> {
        if self.entries.len() + n > MAX_CHECKS {
            return Err(WorkProblem::TooMany);
        }
        Ok(())
    }

    fn push(&mut self, e: Vec<Entry>) -> Result<(), WorkProblem> {
        self.room(e.len())?;
        self.entries.extend(e);
        self.bump();
        Ok(())
    }

    /// Adds a font check.
    pub fn add_font(&mut self, family: &str, folders: Vec<PathBuf>) -> Result<(), WorkProblem> {
        let family = name_ok(family).ok_or(WorkProblem::BadName)?;
        self.push(vec![Entry::new(
            WorkWhat::Font,
            family.clone(),
            Some(Goal::Font { family, folders }),
            None,
        )])
    }

    /// Adds a task the person checks themselves.
    pub fn add_task(&mut self, task: &str) -> Result<(), WorkProblem> {
        let task = name_ok(task).ok_or(WorkProblem::BadName)?;
        self.push(vec![Entry::new(
            WorkWhat::Task,
            task.clone(),
            Some(Goal::PersonCheck { task }),
            None,
        )])
    }

    /// Adds the checks a file needs, told by its content first (`projects::look`): an archive or
    /// a database opens; a project's linked files are there, and each font it names. A file
    /// PCTwin cannot check is kept as "not supported", never passed as fine. What it names past
    /// the limits (a file longer than PCTwin reads, more links or fonts than it takes, fonts past
    /// the most checks one person keeps) is counted on the project's own check, which then still
    /// needs attention.
    pub fn add_file(
        &mut self,
        path: &Path,
        rows: &Catalogue<projects::ProjectRow>,
        font_folders: &[PathBuf],
    ) -> Result<(), WorkProblem> {
        let shown = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());
        let file = |what, goal: Option<Goal>, state| {
            Entry::new(what, crate::guard_views::name(&shown), goal, state)
        };
        let entries = match projects::look(path, rows) {
            Found::Archive => vec![file(
                WorkWhat::Archive,
                Some(Goal::ArchiveOpens {
                    file: path.to_path_buf(),
                }),
                None,
            )],
            Found::Database => vec![file(
                WorkWhat::Database,
                Some(Goal::DatabaseOpens {
                    file: path.to_path_buf(),
                }),
                None,
            )],
            Found::NotSupported => vec![file(
                WorkWhat::LinkedFiles,
                None,
                Some(WorkState::NotSupported),
            )],
            Found::CouldNotRead => vec![file(
                WorkWhat::LinkedFiles,
                None,
                Some(WorkState::CouldNotRead),
            )],
            Found::Project(needs) => {
                // Room for the project's own check, then for as many of its fonts as fit.
                self.room(1)?;
                let fit = needs.fonts.len().min(MAX_CHECKS - self.entries.len() - 1);
                let root = path.parent().map(Path::to_path_buf).unwrap_or_default();
                let mut out = vec![Entry {
                    outside: needs.outside,
                    partly_read: needs.partly_read,
                    links_left: needs.more_links,
                    fonts_left: (needs.fonts.len() - fit) as u64 + needs.more_fonts,
                    ..file(
                        WorkWhat::LinkedFiles,
                        Some(Goal::LinkedFiles {
                            root,
                            links: needs.links,
                        }),
                        None,
                    )
                }];
                for family in needs.fonts.into_iter().take(fit) {
                    out.push(Entry::new(
                        WorkWhat::Font,
                        crate::guard_views::name(&family),
                        Some(Goal::Font {
                            family,
                            folders: font_folders.to_vec(),
                        }),
                        None,
                    ));
                }
                out
            }
        };
        self.push(entries)
    }

    fn checked(&self, generation: u64, index: u32) -> Result<usize, WorkProblem> {
        if generation != self.generation {
            return Err(WorkProblem::Stale);
        }
        let i = index as usize;
        if i >= self.entries.len() {
            return Err(WorkProblem::NoSuchCheck);
        }
        Ok(i)
    }

    /// Removes check `index` of the list `generation`.
    pub fn remove(&mut self, index: u32, generation: u64) -> Result<(), WorkProblem> {
        let i = self.checked(generation, index)?;
        self.entries.remove(i);
        self.bump();
        Ok(())
    }

    /// The readiness checks' own choices, for every entry that has a goal, and where each is.
    fn choices(&self) -> (Choices, Vec<usize>) {
        let mut at = Vec::new();
        let chosen: Vec<Chosen> = self
            .entries
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                let goal = e.goal.clone()?;
                at.push(i);
                Some(Chosen {
                    item: item_of(&e.name, i),
                    owner: Role::Readiness,
                    goal,
                })
            })
            .collect();
        let choices = Choices::picked(self.generation, chosen)
            .expect("never more than the checks' own limit (MAX_CHECKS)");
        (choices, at)
    }

    /// Runs every check PCTwin can do by reading, the insides of files read by `reader` (PCTwin's
    /// parser process). A person's own answer is kept.
    pub fn run(&mut self, reader: &dyn checks::FileReader) {
        // Nothing to check: nothing changes (and nothing is saved).
        if self.entries.is_empty() {
            return;
        }
        let (choices, at) = self.choices();
        for r in checks::run(&choices, reader) {
            let e = &mut self.entries[at[r.index]];
            let waiting = matches!(
                r.outcome,
                Outcome::NotRun {
                    why: NotRun::WaitingForPerson
                }
            );
            if waiting && e.outcome.is_some() {
                continue;
            }
            e.outcome = Some(r.outcome);
        }
        self.bump();
    }

    /// The person's answer to task check `index` of the list `generation`.
    pub fn answer(&mut self, index: u32, generation: u64, worked: bool) -> Result<(), WorkProblem> {
        let i = self.checked(generation, index)?;
        let (choices, at) = self.choices();
        let Some(c) = at.iter().position(|a| *a == i) else {
            return Err(WorkProblem::WrongKind);
        };
        let r = checks::person_answer(&choices, c, generation, worked, Coverage::Whole).map_err(
            |e| match e {
                ChoiceError::StalePlan => WorkProblem::Stale,
                ChoiceError::NoSuchCheck => WorkProblem::NoSuchCheck,
                _ => WorkProblem::WrongKind,
            },
        )?;
        self.entries[i].outcome = Some(r.outcome);
        self.bump();
        Ok(())
    }

    /// What the screens are shown.
    pub fn view(&self) -> WorkChecks {
        let all: Vec<WorkCheck> = self
            .entries
            .iter()
            .enumerate()
            .map(|(i, e)| line(i as u32, e))
            .collect();
        let (checks, more) = crate::guard_views::bounded(all);
        WorkChecks {
            generation: self.generation,
            checks,
            more,
        }
    }
}

/// How much of `e` was looked at, by `c`: `checked` of `of`, counting what was never looked up
/// (links outside the project, links past the limit).
fn coverage(e: &Entry, c: &Coverage) -> (u64, u64) {
    let (checked, of) = match *c {
        Coverage::Whole => {
            let n = match &e.goal {
                Some(Goal::LinkedFiles { links, .. }) => links.len() as u64,
                _ => 1,
            };
            (n, n)
        }
        Coverage::Sample { checked, of } => (checked, of),
    };
    (checked, of + e.outside + e.links_left)
}

/// One entry as the screens show it. Every result carries its label; a check that did not run,
/// that skipped links outside its project, or that did not look at everything (a sample, a file
/// read only in part, links or fonts past the limits) still needs attention.
fn line(index: u32, e: &Entry) -> WorkCheck {
    let base = WorkCheck {
        index,
        what: e.what,
        name: e.name.clone(),
        label: WorkLabel::StillNeedsAttention,
        state: e.state.unwrap_or(WorkState::NotRunYet),
        missing: Vec::new(),
        missing_more: 0,
        not_checked: Vec::new(),
        checked: 0,
        of: 0,
    };
    let Some(outcome) = &e.outcome else {
        return base;
    };
    match outcome {
        Outcome::MachineChecked { coverage: c } => {
            let (checked, of) = coverage(e, c);
            let state = if e.outside > 0 {
                WorkState::OutsideProject
            } else if e.partly() || !c.is_whole() {
                WorkState::PartlyChecked
            } else {
                WorkState::AllThere
            };
            WorkCheck {
                label: if state == WorkState::AllThere {
                    WorkLabel::MachineChecked
                } else {
                    WorkLabel::StillNeedsAttention
                },
                state,
                checked,
                of,
                ..base
            }
        }
        Outcome::PersonConfirmed { coverage: c } => {
            let (checked, of) = coverage(e, c);
            WorkCheck {
                label: WorkLabel::PersonConfirmed,
                state: WorkState::AllThere,
                checked,
                of,
                ..base
            }
        }
        Outcome::NeedsAttention {
            missing,
            coverage: c,
        } => {
            let (checked, of) = coverage(e, c);
            let mut names = Vec::new();
            let mut not_checked = Vec::new();
            let mut state = WorkState::Missing;
            for m in missing {
                match m {
                    Missing::LinkedFile { name } => names.push(crate::guard_views::name(name)),
                    Missing::Font {
                        family,
                        not_checked: kinds,
                    } => {
                        names.push(crate::guard_views::name(family));
                        // The kind as the catalogue names it, else its extension (2.7 made
                        // these typed; the screens show the name as it is).
                        not_checked.extend(kinds.iter().map(|k| {
                            crate::guard_views::name(k.name.as_deref().unwrap_or(&k.ext))
                        }));
                    }
                    Missing::Damaged { .. } => state = WorkState::Damaged,
                    Missing::NotThere => state = WorkState::NotThere,
                    Missing::PersonSaidNo => state = WorkState::YouSaidNo,
                }
            }
            let missing_more = names.len().saturating_sub(MAX_MISSING) as u64;
            names.truncate(MAX_MISSING);
            not_checked.sort();
            not_checked.dedup();
            WorkCheck {
                state,
                missing: names,
                missing_more,
                not_checked,
                checked,
                of,
                ..base
            }
        }
        Outcome::NotRun { why } => WorkCheck {
            state: match why {
                NotRun::WaitingForPerson => WorkState::WaitingForYou,
                NotRun::OutsideTheProject => WorkState::OutsideProject,
                NotRun::CouldNotRead | NotRun::CouldNotBeChecked => WorkState::CouldNotRead,
                NotRun::NeedsItsOwnApproval | NotRun::NotBuiltYet => WorkState::NotSupported,
            },
            ..base
        },
    }
}

/// The folders this laptop's system keeps fonts in: for everyone, and for the signed-in person.
/// The system's own places, not a list of fonts.
pub fn font_folders() -> Vec<PathBuf> {
    let env = |k: &str| std::env::var_os(k).map(PathBuf::from);
    let mut out = Vec::new();
    if cfg!(windows) {
        if let Some(w) = env("WINDIR") {
            out.push(w.join("Fonts"));
        }
        if let Some(l) = env("LOCALAPPDATA") {
            out.push(l.join("Microsoft").join("Windows").join("Fonts"));
        }
    } else if cfg!(target_os = "macos") {
        out.push(PathBuf::from("/System/Library/Fonts"));
        out.push(PathBuf::from("/Library/Fonts"));
        if let Some(h) = env("HOME") {
            out.push(h.join("Library").join("Fonts"));
        }
    } else {
        out.push(PathBuf::from("/usr/share/fonts"));
        out.push(PathBuf::from("/usr/local/share/fonts"));
        if let Some(h) = env("HOME") {
            out.push(h.join(".local").join("share").join("fonts"));
            out.push(h.join(".fonts"));
        }
    }
    out
}

/* ---------- The program's list, kept in PCTwin's app data folder ---------- */

const FILE: &str = "work-checks.json";

/// The biggest list file read or written (200 checks, each with up to 10,000 linked file names,
/// fit well inside it); a bigger one is never read whole into memory.
const MAX_FILE_BYTES: u64 = 128 * 1024 * 1024;

fn state() -> &'static Mutex<Option<Work>> {
    static WORK: OnceLock<Mutex<Option<Work>>> = OnceLock::new();
    WORK.get_or_init(|| Mutex::new(None))
}

/// The list kept at `path`: a file that is missing, too big, not a list or over the limit is an
/// empty list.
fn load(path: &Path) -> Work {
    use std::io::Read;
    let mut bytes = Vec::new();
    let read = std::fs::File::open(path)
        .and_then(|f| f.take(MAX_FILE_BYTES + 1).read_to_end(&mut bytes))
        .is_ok();
    if !read || bytes.len() as u64 > MAX_FILE_BYTES {
        return Work::default();
    }
    serde_json::from_slice::<Work>(&bytes)
        .ok()
        .filter(|w| w.entries.len() <= MAX_CHECKS)
        .unwrap_or_default()
}

/// Keeps `work` at `path` whole or not at all: written beside it first, then put in its place in
/// one step, so a crash or a full disk part way leaves the last whole list.
fn save(path: &Path, work: &Work) -> std::io::Result<()> {
    use std::io::Write;
    let bytes = serde_json::to_vec(work).map_err(std::io::Error::other)?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(std::io::Error::other("the list is too big to keep"));
    }
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d)?;
    }
    let mut part = path.as_os_str().to_owned();
    part.push(".part");
    let part = PathBuf::from(part);
    let written = std::fs::File::create(&part).and_then(|mut f| {
        f.write_all(&bytes)?;
        f.sync_all()
    });
    if let Err(e) = written.and_then(|()| std::fs::rename(&part, path)) {
        let _ = std::fs::remove_file(&part);
        return Err(e);
    }
    Ok(())
}

/// Runs `f` on the list in `state`, read from `path` the first time and kept there whenever it
/// changes.
fn with_in<T>(
    state: &Mutex<Option<Work>>,
    path: Option<&Path>,
    f: impl FnOnce(&mut Work) -> T,
) -> T {
    let mut guard = state.lock().unwrap_or_else(|e| e.into_inner());
    let work = guard.get_or_insert_with(|| path.map(load).unwrap_or_default());
    let before = work.generation;
    let out = f(work);
    if work.generation != before
        && let Some(p) = path
    {
        let _ = save(p, work);
    }
    out
}

fn with<R: Runtime, T>(app: &AppHandle<R>, f: impl FnOnce(&mut Work) -> T) -> T {
    let path = app.path().app_data_dir().ok().map(|d| d.join(FILE));
    with_in(state(), path.as_deref(), f)
}

/// Runs every check on a copy of the list, with the list itself free meanwhile (reading
/// thousands of files takes a while); the results are kept only if the list did not change
/// while they ran (`between` runs while they run: for tests).
fn run_in(
    state: &Mutex<Option<Work>>,
    path: Option<&Path>,
    reader: &dyn checks::FileReader,
    between: impl FnOnce(),
) -> Result<WorkChecks, WorkProblem> {
    let mut copy = with_in(state, path, |w| w.clone());
    let started = copy.generation;
    copy.run(reader);
    between();
    with_in(state, path, |w| {
        if w.generation != started {
            return Err(WorkProblem::Stale);
        }
        *w = copy;
        Ok(w.view())
    })
}

/// The person's checks, with their latest results.
#[tauri::command]
pub fn work_checks<R: Runtime>(app: AppHandle<R>) -> WorkChecks {
    with(&app, |w| w.view())
}

/// Adds a check. A file is picked in the system's own file picker: the screens never send a path.
#[tauri::command]
pub async fn work_check_add<R: Runtime>(
    app: AppHandle<R>,
    pick: WorkPick,
) -> Result<WorkChecks, WorkProblem> {
    match pick {
        WorkPick::File {} => {
            let file = tauri::async_runtime::spawn_blocking(|| rfd::FileDialog::new().pick_file())
                .await
                .map_err(|_| WorkProblem::Failed)?
                .ok_or(WorkProblem::NotChosen)?;
            let folders = font_folders();
            with(&app, |w| {
                w.add_file(&file, projects::shipped(), &folders)?;
                Ok(w.view())
            })
        }
        WorkPick::Font { family } => {
            let folders = font_folders();
            with(&app, |w| {
                w.add_font(&family, folders)?;
                Ok(w.view())
            })
        }
        WorkPick::Task { task } => with(&app, |w| {
            w.add_task(&task)?;
            Ok(w.view())
        }),
    }
}

/// Runs every check that PCTwin can do by reading (never running anything).
#[tauri::command]
pub async fn work_check_run<R: Runtime>(app: AppHandle<R>) -> Result<WorkChecks, WorkProblem> {
    // Reading thousands of linked files or font files takes a moment: off the screens' thread,
    // and with the list free meanwhile.
    tauri::async_runtime::spawn_blocking(move || {
        let path = app.path().app_data_dir().ok().map(|d| d.join(FILE));
        run_in(
            state(),
            path.as_deref(),
            crate::parser(crate::Files::WorkChecks),
            || {},
        )
    })
    .await
    .map_err(|_| WorkProblem::Failed)?
}

/// The person's answer to a task check: it worked, or it did not.
#[tauri::command]
pub fn work_check_answer<R: Runtime>(
    app: AppHandle<R>,
    index: u32,
    generation: u64,
    worked: bool,
) -> Result<WorkChecks, WorkProblem> {
    with(&app, |w| {
        w.answer(index, generation, worked)?;
        Ok(w.view())
    })
}

/// Removes a check.
#[tauri::command]
pub fn work_check_remove<R: Runtime>(
    app: AppHandle<R>,
    index: u32,
    generation: u64,
) -> Result<WorkChecks, WorkProblem> {
    with(&app, |w| {
        w.remove(index, generation)?;
        Ok(w.view())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The test's file reader, standing in for PCTwin's parser process (not beside a test): fonts
    /// and databases are never read here; an archive answers as the parser does for the one cut
    /// archive these tests make (its end record is missing).
    struct NoReader;
    impl checks::FileReader for NoReader {
        fn font(&self, _: &std::fs::File, _: u64) -> Result<checks::FontNames, String> {
            Err("not read in tests".into())
        }
        fn archive(
            &self,
            _: &std::fs::File,
            _: u64,
        ) -> Result<pctwin_readiness::checks::Shape, String> {
            Ok(pctwin_readiness::checks::Shape::Damaged(
                pctwin_readiness::checks::Damage::NoArchiveEnd,
            ))
        }
        fn database(
            &self,
            _: &std::fs::File,
            _: u64,
        ) -> Result<pctwin_readiness::checks::Shape, String> {
            Err("not read in tests".into())
        }
    }
    static NO_READER: NoReader = NoReader;

    fn put(path: &Path, bytes: &[u8]) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }

    fn states(w: &Work) -> Vec<(WorkWhat, WorkLabel, WorkState)> {
        w.view()
            .checks
            .iter()
            .map(|c| (c.what, c.label, c.state))
            .collect()
    }

    /// Task List 2.9 (R17): a project's linked files and fonts, an archive, a database and a task
    /// each end with one of the three labels; nothing is "fine" that was not checked.
    #[test]
    fn every_check_ends_with_one_of_the_three_labels_honestly() {
        use WorkLabel::*;
        use WorkState::*;
        use WorkWhat::*;
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("project");
        put(&p.join("images/a.png"), b"png");
        put(
            &p.join("Poster.svg"),
            br#"<svg><image href="images/a.png"/><image href="images/b.png"/>
                <text font-family="NoSuchFamilyAnywhere, serif">x</text></svg>"#,
        );
        put(
            &p.join("Fine.svg"),
            br#"<svg><image href="images/a.png"/></svg>"#,
        );
        put(
            &p.join("Outside.svg"),
            br#"<svg><image href="images/a.png"/><image href="../x.png"/></svg>"#,
        );
        put(&p.join("cut.zip"), b"PK\x03\x04 and then nothing");
        put(&p.join("drawing.xyz"), b"something");
        let fonts = t.path().join("fonts");
        std::fs::create_dir_all(&fonts).unwrap();
        let mut w = Work::default();
        let rows = projects::shipped();
        for f in [
            "Poster.svg",
            "Fine.svg",
            "Outside.svg",
            "cut.zip",
            "drawing.xyz",
        ] {
            w.add_file(&p.join(f), rows, std::slice::from_ref(&fonts))
                .unwrap();
        }
        w.add_file(&p.join("gone.svg"), rows, &[]).unwrap();
        w.add_task("Print my boarding pass").unwrap();
        // Before running, nothing is labelled checked.
        assert!(
            w.view()
                .checks
                .iter()
                .all(|c| c.label == StillNeedsAttention)
        );
        w.run(&NO_READER);
        assert_eq!(
            states(&w),
            [
                (LinkedFiles, StillNeedsAttention, Missing),
                (Font, StillNeedsAttention, Missing),
                (LinkedFiles, MachineChecked, AllThere),
                (LinkedFiles, StillNeedsAttention, OutsideProject),
                (Archive, StillNeedsAttention, Damaged),
                (LinkedFiles, StillNeedsAttention, NotSupported),
                (LinkedFiles, StillNeedsAttention, CouldNotRead),
                (Task, StillNeedsAttention, WaitingForYou),
            ]
        );
        let v = w.view();
        assert_eq!(v.checks[0].missing, ["images/b.png"]);
        assert_eq!(v.checks[1].missing, ["NoSuchFamilyAnywhere"]);
        assert_eq!((v.checks[2].checked, v.checks[2].of), (1, 1));
        // One link looked up, one outside: never "all of it".
        assert_eq!((v.checks[3].checked, v.checks[3].of), (1, 2));
        // The person's own answer: confirmed, or still needs attention; kept across a new run.
        let g = w.view().generation;
        w.answer(7, g, true).unwrap();
        w.run(&NO_READER);
        assert_eq!(states(&w)[7], (Task, PersonConfirmed, AllThere));
        let confirmed = &w.view().checks[7];
        assert_eq!(
            (confirmed.checked, confirmed.of),
            (1, 1),
            "the person's yes counts"
        );
        let g = w.view().generation;
        w.answer(7, g, false).unwrap();
        assert_eq!(states(&w)[7], (Task, StillNeedsAttention, YouSaidNo));
        // Only a task takes an answer.
        let g = w.view().generation;
        assert_eq!(w.answer(2, g, true), Err(WorkProblem::WrongKind));
        assert_eq!(w.answer(5, g, true), Err(WorkProblem::WrongKind));
    }

    #[test]
    fn an_answer_or_removal_for_another_version_is_refused() {
        let mut w = Work::default();
        w.add_task("Open my accounts").unwrap();
        let g = w.view().generation;
        w.add_task("Open my invoices").unwrap();
        assert_eq!(w.answer(0, g, true), Err(WorkProblem::Stale));
        assert_eq!(w.remove(0, g), Err(WorkProblem::Stale));
        let g = w.view().generation;
        assert_eq!(w.answer(9, g, true), Err(WorkProblem::NoSuchCheck));
        assert_eq!(w.remove(2, g), Err(WorkProblem::NoSuchCheck));
        w.remove(0, g).unwrap();
        let v = w.view();
        assert_eq!(v.checks.len(), 1);
        assert_eq!(v.checks[0].name, "Open my invoices");
        assert_eq!(v.checks[0].index, 0);
    }

    #[test]
    fn names_are_checked_and_the_list_is_bounded() {
        let mut w = Work::default();
        assert_eq!(w.add_task("   "), Err(WorkProblem::BadName));
        assert_eq!(w.add_task("a\u{7}b"), Err(WorkProblem::BadName));
        assert_eq!(
            w.add_font(&"x".repeat(MAX_NAME_CHARS + 1), vec![]),
            Err(WorkProblem::BadName)
        );
        w.add_font(&"x".repeat(MAX_NAME_CHARS), vec![]).unwrap();
        assert_eq!(w.view().checks[0].name.chars().count(), MAX_NAME_CHARS);
        w.add_task("  trimmed  ").unwrap();
        assert_eq!(w.view().checks[1].name, "trimmed");
        for i in 2..MAX_CHECKS {
            w.add_task(&format!("t{i}")).unwrap();
        }
        assert_eq!(w.add_task("one more"), Err(WorkProblem::TooMany));
        let before = w.view().generation;
        w.run(&NO_READER);
        assert_ne!(w.view().generation, before);
        let mut empty = Work::default();
        empty.run(&NO_READER);
        assert_eq!(empty.view().generation, 0, "an empty list is never saved");
    }

    #[test]
    fn a_found_font_is_machine_checked_and_a_database_opens() {
        let t = tempfile::tempdir().unwrap();
        let db = t.path().join("recipes.db");
        // A one-page SQLite database header: page size 4096, one page.
        let mut page = vec![0u8; 4096];
        page[..16].copy_from_slice(b"SQLite format 3\0");
        page[16] = 0x10;
        page[17] = 0x00;
        page[18] = 1;
        page[19] = 1;
        page[21] = 64;
        page[22] = 32;
        page[23] = 32;
        page[31] = 1;
        put(&db, &page);
        let mut w = Work::default();
        w.add_file(&db, projects::shipped(), &[]).unwrap();
        w.run(&NO_READER);
        let c = &w.view().checks[0];
        assert_eq!(c.what, WorkWhat::Database);
        assert_ne!(c.state, WorkState::NotRunYet);
    }

    /// Fresh-context review F3: a file read only in part, links past the most one check takes,
    /// and fonts past the limits are carried into the project's check, which then still needs
    /// attention ("not everything was looked at"), never "machine-checked"; the counts say so.
    #[test]
    fn what_was_not_looked_at_keeps_a_check_needing_attention() {
        use WorkLabel::*;
        use WorkState::*;
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("project");
        put(&p.join("a.png"), b"png");
        let rows = projects::shipped();
        // Longer than PCTwin reads: the link it found is there, but the rest was not read.
        let mut long = br#"<svg><image href="a.png"/>"#.to_vec();
        long.resize(projects::MAX_PROJECT_BYTES as usize + 10, b' ');
        long.extend_from_slice(b"</svg>");
        put(&p.join("Long.svg"), &long);
        let mut w = Work::default();
        w.add_file(&p.join("Long.svg"), rows, &[]).unwrap();
        w.run(&NO_READER);
        let c = &w.view().checks[0];
        assert_eq!((c.label, c.state), (StillNeedsAttention, PartlyChecked));
        assert_eq!((c.checked, c.of), (1, 1));
        // More links than one check takes: the ones past it are counted, never looked up.
        let mut many = String::from("<svg>");
        for i in 0..projects::MAX_PROJECT_LINKS + 3 {
            many.push_str(&format!("<image href=\"none/{i}.png\"/>"));
        }
        many.push_str("</svg>");
        put(&p.join("Many.svg"), many.as_bytes());
        let mut w = Work::default();
        w.add_file(&p.join("Many.svg"), rows, &[]).unwrap();
        w.run(&NO_READER);
        let c = &w.view().checks[0];
        assert_eq!((c.label, c.state), (StillNeedsAttention, Missing));
        assert_eq!(c.of, c.checked + 3);
        // Fonts past what one person keeps: the project's own check fits, the fonts that do not
        // are counted on it.
        let mut fonts = String::from(r#"<svg><image href="a.png"/>"#);
        for i in 0..MAX_CHECKS + 4 {
            fonts.push_str(&format!("<text font-family=\"Family{i}\">x</text>"));
        }
        fonts.push_str("</svg>");
        put(&p.join("Fonts.svg"), fonts.as_bytes());
        let mut w = Work::default();
        w.add_file(&p.join("Fonts.svg"), rows, &[]).unwrap();
        assert_eq!(w.entries.len(), MAX_CHECKS);
        assert_eq!(w.entries[0].fonts_left, 1 + 4);
        w.run(&NO_READER);
        let c = &w.view().checks[0];
        assert_eq!((c.label, c.state), (StillNeedsAttention, PartlyChecked));
        // A list with no room left for the project's own check refuses it.
        assert_eq!(
            w.add_file(&p.join("Fonts.svg"), rows, &[]),
            Err(WorkProblem::TooMany)
        );
        // A whole check of everything is machine-checked.
        put(&p.join("Fine.svg"), br#"<svg><image href="a.png"/></svg>"#);
        let mut w = Work::default();
        w.add_file(&p.join("Fine.svg"), rows, &[]).unwrap();
        w.run(&NO_READER);
        let c = &w.view().checks[0];
        assert_eq!((c.label, c.state), (MachineChecked, AllThere));
        // A sample is never machine-checked, even with nothing else left out.
        let e = Entry {
            outcome: Some(Outcome::MachineChecked {
                coverage: Coverage::Sample { checked: 3, of: 9 },
            }),
            ..Entry::new(WorkWhat::LinkedFiles, "x".into(), None, None)
        };
        let c = line(0, &e);
        assert_eq!(
            (c.label, c.state, c.checked, c.of),
            (StillNeedsAttention, PartlyChecked, 3, 9)
        );
        // A list saved before these counts existed still loads.
        let old = r#"{"generation":1,"entries":[{"what":"task","name":"t","goal":null,"state":null,"outside":0,"outcome":null}]}"#;
        assert_eq!(serde_json::from_str::<Work>(old).unwrap().entries.len(), 1);
    }

    /// Fresh-context review F9: the list is kept whole or not at all, a file too big is never
    /// read, and checks run with the list free; results for a list that changed meanwhile are
    /// refused, never put over the newer list.
    #[test]
    fn the_list_is_kept_whole_and_free_while_checks_run() {
        let t = tempfile::tempdir().unwrap();
        let path = t.path().join("data").join(FILE);
        let state = Mutex::new(None);
        with_in(&state, Some(&path), |w| w.add_task("Print").unwrap());
        assert!(path.exists());
        let mut part = path.as_os_str().to_owned();
        part.push(".part");
        assert!(!PathBuf::from(&part).exists(), "nothing left beside it");
        assert_eq!(load(&path).entries.len(), 1);
        // A failed write leaves the last whole list (here: the place is a folder).
        std::fs::create_dir(PathBuf::from(&part)).unwrap();
        let mut w = load(&path);
        w.add_task("Scan").unwrap();
        assert!(save(&path, &w).is_err());
        assert_eq!(load(&path).entries.len(), 1);
        std::fs::remove_dir(PathBuf::from(&part)).unwrap();
        // Too big: never read whole, an empty list instead.
        // (A whole list, padded past the bound with spaces JSON allows.)
        let big = t.path().join("big.json");
        let mut bytes = std::fs::read(&path).unwrap();
        assert_eq!(load(&path).entries.len(), 1);
        bytes.resize(MAX_FILE_BYTES as usize + 1, b' ');
        std::fs::write(&big, &bytes).unwrap();
        assert_eq!(load(&big).entries.len(), 0);
        drop(bytes);
        std::fs::remove_file(&big).unwrap();
        // While the checks run the list is free; a change meanwhile wins.
        let r = run_in(&state, Some(&path), &NO_READER, || {
            with_in(&state, Some(&path), |w| w.add_task("Call").unwrap());
        });
        assert_eq!(r, Err(WorkProblem::Stale));
        let names: Vec<String> = with_in(&state, None, |w| w.view())
            .checks
            .into_iter()
            .map(|c| c.name)
            .collect();
        assert_eq!(names, ["Print", "Call"]);
        assert_eq!(load(&path).entries.len(), 2);
        // Nothing changed meanwhile: the results are kept, and saved.
        let v = run_in(&state, Some(&path), &NO_READER, || {}).unwrap();
        assert_eq!(v.checks[0].state, WorkState::WaitingForYou);
        assert!(load(&path).entries[0].outcome.is_some());
    }

    #[test]
    fn work_items_are_stable_per_name_and_place() {
        assert_eq!(item_of("a", 0), item_of("a", 0));
        assert_ne!(item_of("a", 0), item_of("a", 1));
        assert_ne!(item_of("a", 0), item_of("b", 0));
        assert!(!font_folders().is_empty());
    }
}
