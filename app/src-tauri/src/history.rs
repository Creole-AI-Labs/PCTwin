//! My PCTwin history (F08): the moves and Quick Sends done on this laptop, each with its report
//! or receipt, kept in this laptop's app data folder only (no account, nothing sent anywhere).
//! Entries are written here, from the engine's own results when a move or a send ends, never by
//! the screens: so "finished" is said only when the whole thing finished. Export saves a plain
//! copy (one line per thing, readable without PCTwin) where the person chooses.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime};

use crate::moving::{MoveProblem, ReportItem};
use crate::pairing::LabelView;
use crate::quicksend::ReceiptView;

/// At most this many entries are kept; the oldest go first.
const MAX_ENTRIES: usize = 500;

/// One thing done on this laptop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: String,
    /// "move", "send" or "receive".
    pub kind: String,
    /// When it ended, in UTC (RFC 3339).
    pub when: String,
    /// The old laptop's label as pairing showed it (the screens' label shape), when known.
    pub other: Option<serde_json::Value>,
    /// "finished", "steps-left", "stopped" (moves) or "finished", "stopped", "partial" (sends).
    pub state: String,
    /// A move with steps left: how many things still need the person.
    pub steps_left: u64,
}

/// What an entry kept: the move's report, or the send's receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Kept {
    Report { items: Vec<ReportItemStored> },
    Receipt { receipt: ReceiptStored },
}

/// A report line as kept (the same shape the screens read).
pub type ReportItemStored = serde_json::Value;
/// A receipt as kept (the same shape the screens read).
pub type ReceiptStored = serde_json::Value;

fn dir_of<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    app.path().app_data_dir().ok().map(|d| d.join("history"))
}

fn read_index(dir: &Path) -> Vec<Entry> {
    std::fs::read(dir.join("index.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
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

/// A send's history state: "finished" only when every file is in place.
pub fn send_state(receipt: &ReceiptView) -> &'static str {
    use crate::quicksend::OutcomeView;
    let whole = !receipt.lines.is_empty()
        && receipt.lines.iter().all(|l| {
            matches!(
                l.outcome,
                OutcomeView::Arrived { .. } | OutcomeView::AlreadyThere { .. }
            )
        });
    if whole {
        "finished"
    } else if receipt.stopped {
        "stopped"
    } else {
        "partial"
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

/// Keeps a send's receipt.
pub fn receipt_kept(r: &ReceiptView) -> Kept {
    Kept::Receipt {
        receipt: serde_json::to_value(r).unwrap_or(serde_json::Value::Null),
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
    if !read_index(&dir).iter().any(|e| e.id == id) {
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

    #[test]
    fn a_send_is_finished_only_when_every_file_is_in_place() {
        use crate::quicksend::{LineView, OutcomeView};
        let line = |o| LineView {
            path: "a".into(),
            outcome: o,
        };
        let r = |lines, stopped| ReceiptView {
            side: "sender",
            lines,
            stopped,
        };
        assert_eq!(
            send_state(&r(
                vec![line(OutcomeView::Arrived {
                    landed_as: "a".into()
                })],
                false
            )),
            "finished"
        );
        assert_eq!(send_state(&r(Vec::new(), false)), "partial");
        assert_eq!(
            send_state(&r(
                vec![line(OutcomeView::NeedsAction {
                    landed_as: "a".into(),
                    why: "x".into(),
                    word: None,
                })],
                false
            )),
            "partial"
        );
        assert_eq!(
            send_state(&r(vec![line(OutcomeView::Declined)], true)),
            "stopped"
        );
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
            state: "finished".into(),
            steps_left: 0,
        };
        assert_eq!(
            csv(&[e]),
            "when,what,state,steps left\n2026-10-22T00:00:00Z,move,finished,0\n"
        );
    }
}
