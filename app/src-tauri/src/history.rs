//! My PCTwin history (F08): the moves done on this laptop, each with its report, kept in this
//! laptop's app data folder only (no account, nothing sent anywhere). Entries are written here,
//! from the engine's own results when a move ends, never by the screens: so "finished" is said
//! only when the whole move finished. Export saves a plain copy (one line per move, readable
//! without PCTwin) where the person chooses.
//!
//! Quick Send keeps nothing here: what was sent or received is not remembered (decided 26 October
//! 2026). Entries an earlier version kept for sends are removed the first time history is read.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime};

use crate::moving::{MoveProblem, ReportItem};
use crate::pairing::LabelView;

/// At most this many entries are kept; the oldest go first.
const MAX_ENTRIES: usize = 500;

/// One thing done on this laptop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: String,
    /// "move" (earlier versions also kept "send" and "receive"; see [`read_index`]).
    pub kind: String,
    /// When it ended, in UTC (RFC 3339).
    pub when: String,
    /// The old laptop's label as pairing showed it (the screens' label shape), when known.
    pub other: Option<serde_json::Value>,
    /// The new laptop's name, when known (entries before 28 October 2026 have none).
    #[serde(default)]
    pub new_name: Option<serde_json::Value>,
    /// "finished", "steps-left" or "stopped".
    pub state: String,
    /// A move with steps left: how many things still need the person.
    pub steps_left: u64,
}

/// What an entry kept: the move's report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Kept {
    Report { items: Vec<ReportItemStored> },
}

/// A report line as kept (the same shape the screens read).
pub type ReportItemStored = serde_json::Value;

fn dir_of<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    app.path().app_data_dir().ok().map(|d| d.join("history"))
}

/// Whether `id` is an entry ID as [`add`] makes them (digits, then a dash, then digits): the only
/// names this module ever turns into a file name, so an index changed by hand can never point at
/// a file outside the history folder ("../x", an absolute path).
fn plain_id(id: &str) -> bool {
    !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit() || b == b'-')
}

/// The moves kept here. A Quick Send an earlier version kept is removed, with what it kept, the
/// first time the list is read: sends are not remembered.
fn read_index(dir: &Path) -> Vec<Entry> {
    let index: Vec<Entry> = std::fs::read(dir.join("index.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    let (moves, sends): (Vec<Entry>, Vec<Entry>) =
        index.into_iter().partition(|e| e.kind == "move");
    if !sends.is_empty()
        && let Ok(bytes) = serde_json::to_vec(&moves)
        && write_whole(&dir.join("index.json"), &bytes).is_ok()
    {
        for e in sends.iter().filter(|e| plain_id(&e.id)) {
            let _ = std::fs::remove_file(dir.join(format!("{}.json", e.id)));
        }
    }
    moves
}

/// Writes `bytes` to `path` in one step, so a crash never leaves half of it.
fn write_whole(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("part");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

fn now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    rfc3339(secs)
}

/// Seconds since 1970 as an RFC 3339 UTC time (no time zone tables needed).
pub fn rfc3339(secs: u64) -> String {
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = i64::try_from(days).unwrap_or(0) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}

/// Adds an entry with what it kept. Failing to keep history never fails a move.
pub fn add<R: Runtime>(
    app: &AppHandle<R>,
    kind: &str,
    other: Option<&LabelView>,
    new: Option<&LabelView>,
    state: &str,
    steps_left: u64,
    kept: &Kept,
) {
    let Some(dir) = dir_of(app) else { return };
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let mut index = read_index(&dir);
    let id = format!(
        "{}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0),
        index.len()
    );
    let Ok(bytes) = serde_json::to_vec(kept) else {
        return;
    };
    if write_whole(&dir.join(format!("{id}.json")), &bytes).is_err() {
        return;
    }
    index.push(Entry {
        id,
        kind: kind.to_string(),
        when: now(),
        other: other.and_then(|l| serde_json::to_value(l).ok()),
        new_name: new.and_then(|l| serde_json::to_value(l).ok()),
        state: state.to_string(),
        steps_left,
    });
    while index.len() > MAX_ENTRIES {
        let gone = index.remove(0);
        let _ = std::fs::remove_file(dir.join(format!("{}.json", gone.id)));
    }
    if let Ok(bytes) = serde_json::to_vec(&index) {
        let _ = write_whole(&dir.join("index.json"), &bytes);
    }
}

/// A move's history state from its outcome.
pub fn move_state(outcome: &crate::moving::MoveOutcome) -> (&'static str, u64) {
    let left = outcome.by_status.needs_action + outcome.by_status.failed + outcome.not_copied;
    if outcome.stopped {
        ("stopped", left)
    } else if left > 0 {
        ("steps-left", left)
    } else {
        ("finished", 0)
    }
}

/// Keeps a move's report.
pub fn report_kept(items: &[ReportItem]) -> Kept {
    Kept::Report {
        items: items
            .iter()
            .filter_map(|i| serde_json::to_value(i).ok())
            .collect(),
    }
}

/* ---------- Commands ---------- */

#[tauri::command]
pub fn history_list<R: Runtime>(app: AppHandle<R>) -> Vec<Entry> {
    dir_of(&app).map(|d| read_index(&d)).unwrap_or_default()
}

/// What one entry kept, by its ID (only an ID from the list).
#[tauri::command]
pub fn history_open<R: Runtime>(app: AppHandle<R>, id: String) -> Result<Kept, MoveProblem> {
    let dir = dir_of(&app).ok_or(MoveProblem::Failed)?;
    if !plain_id(&id) || !read_index(&dir).iter().any(|e| e.id == id) {
        return Err(MoveProblem::Failed);
    }
    let bytes = std::fs::read(dir.join(format!("{id}.json"))).map_err(|_| MoveProblem::Failed)?;
    serde_json::from_slice(&bytes).map_err(|_| MoveProblem::Failed)
}

/// Saves a plain copy of the history (one line per entry) in a folder the person chooses in the
/// system's picker. Returns where it went, or `None` when they chose nowhere.
#[tauri::command]
pub async fn history_export<R: Runtime>(app: AppHandle<R>) -> Result<Option<String>, MoveProblem> {
    let entries = history_list(app);
    let folder = tauri::async_runtime::spawn_blocking(|| rfd::FileDialog::new().pick_folder())
        .await
        .map_err(|_| MoveProblem::Failed)?;
    let Some(folder) = folder else {
        return Ok(None);
    };
    let path = save_new(&folder, csv(&entries).as_bytes()).map_err(|_| MoveProblem::Failed)?;
    Ok(Some(path.display().to_string()))
}

/// Writes `bytes` as a new file in `folder`, never over one already there: "PCTwin history.csv",
/// or "PCTwin history (2).csv" and so on when that name is taken. Made with create-new, so a file
/// that appears in between is not replaced either.
fn save_new(folder: &std::path::Path, bytes: &[u8]) -> std::io::Result<std::path::PathBuf> {
    use std::io::Write;
    for n in 1..=999u32 {
        let name = if n == 1 {
            "PCTwin history.csv".to_string()
        } else {
            format!("PCTwin history ({n}).csv")
        };
        let path = folder.join(name);
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => {
                file.write_all(bytes)?;
                file.sync_all()?;
                return Ok(path);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(std::io::ErrorKind::AlreadyExists.into())
}

/// The history as plain CSV: when, what, state, steps left.
pub fn csv(entries: &[Entry]) -> String {
    let mut out = String::from("when,what,state,steps left\n");
    for e in entries {
        out.push_str(&format!(
            "{},{},{},{}\n",
            e.when, e.kind, e.state, e.steps_left
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_export_never_replaces_a_file_already_there() {
        let dir =
            std::env::temp_dir().join(format!("pctwin-history-export-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("PCTwin history.csv"), b"mine").unwrap();
        let first = save_new(&dir, b"one").unwrap();
        let second = save_new(&dir, b"two").unwrap();
        assert_eq!(first.file_name().unwrap(), "PCTwin history (2).csv");
        assert_eq!(second.file_name().unwrap(), "PCTwin history (3).csv");
        assert_eq!(
            std::fs::read(dir.join("PCTwin history.csv")).unwrap(),
            b"mine"
        );
        assert_eq!(std::fs::read(&first).unwrap(), b"one");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn times_are_written_as_utc_dates() {
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339(1_792_627_200), "2026-10-22T00:00:00Z");
        assert_eq!(rfc3339(951_782_400), "2000-02-29T00:00:00Z");
    }

    /// Quick Send keeps nothing (26 October 2026): sends an earlier version kept are removed,
    /// with what they kept, the first time history is read; moves stay as they were.
    #[test]
    fn sends_kept_by_an_earlier_version_are_forgotten_and_moves_stay() {
        let dir = std::env::temp_dir().join(format!("pctwin-history-sends-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let entry = |id: &str, kind: &str| Entry {
            id: id.into(),
            kind: kind.into(),
            when: "2026-10-25T00:00:00Z".into(),
            other: None,
            new_name: None,
            state: "finished".into(),
            steps_left: 0,
        };
        let old = vec![
            entry("1", "move"),
            entry("2", "send"),
            entry("3", "receive"),
        ];
        std::fs::write(dir.join("index.json"), serde_json::to_vec(&old).unwrap()).unwrap();
        for id in ["1", "2", "3"] {
            std::fs::write(dir.join(format!("{id}.json")), b"{}").unwrap();
        }
        assert_eq!(read_index(&dir), vec![entry("1", "move")]);
        assert!(dir.join("1.json").exists(), "the move's report stays");
        assert!(!dir.join("2.json").exists(), "what a send kept is gone");
        assert!(!dir.join("3.json").exists(), "what a receive kept is gone");
        let on_disk: Vec<Entry> =
            serde_json::from_slice(&std::fs::read(dir.join("index.json")).unwrap()).unwrap();
        assert_eq!(
            on_disk,
            vec![entry("1", "move")],
            "the list itself forgets them"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// PR #109 review: an index changed by hand cannot make the clean-up delete, or Open read, a
    /// file outside the history folder.
    #[test]
    fn only_plain_entry_ids_become_file_names() {
        let root = std::env::temp_dir().join(format!("pctwin-history-ids-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = root.join("history");
        std::fs::create_dir_all(&dir).unwrap();
        let outside = root.join("victim.json");
        std::fs::write(&outside, b"keep me").unwrap();
        let entry = |id: &str| Entry {
            id: id.into(),
            kind: "send".into(),
            when: "2026-10-25T00:00:00Z".into(),
            other: None,
            new_name: None,
            state: "finished".into(),
            steps_left: 0,
        };
        let absolute = root.join("victim").display().to_string();
        let old = vec![entry("../victim"), entry(&absolute)];
        std::fs::write(dir.join("index.json"), serde_json::to_vec(&old).unwrap()).unwrap();
        assert!(read_index(&dir).is_empty());
        assert!(
            outside.exists(),
            "nothing outside the history folder is removed"
        );
        assert!(plain_id("1792627200000-3"));
        assert!(!plain_id("../victim") && !plain_id(&absolute) && !plain_id(""));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_move_with_anything_left_is_never_finished() {
        let mut o = crate::moving::MoveOutcome::default();
        o.by_status.copied = 5;
        assert_eq!(move_state(&o), ("finished", 0));
        o.by_status.failed = 1;
        assert_eq!(move_state(&o), ("steps-left", 1));
        o.stopped = true;
        o.not_copied = 2;
        assert_eq!(move_state(&o), ("stopped", 3));
    }

    #[test]
    fn the_export_is_plain_lines() {
        let e = Entry {
            id: "1".into(),
            kind: "move".into(),
            when: "2026-10-22T00:00:00Z".into(),
            other: None,
            new_name: None,
            state: "finished".into(),
            steps_left: 0,
        };
        assert_eq!(
            csv(&[e]),
            "when,what,state,steps left\n2026-10-22T00:00:00Z,move,finished,0\n"
        );
    }
}
