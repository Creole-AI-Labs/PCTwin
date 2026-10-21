//! Quick Send for the screens (Q01 to Q14): chosen files and folders to a laptop nearby, over the
//! same pairing, sealed link and safety gate as a move. The screens never name a path: files and
//! folders are chosen in the system's own picker here, and the screens get names, sizes and
//! opaque IDs only. The receiver chooses its folder the same way, and nothing received is opened.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use pctwin_move::Live;
use pctwin_quicksend::{Offer, Outcome, Receipt};
use pctwin_record::ItemId;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Runtime, State};
use tokio::sync::oneshot;

use crate::moving::{MoveProblem, journal_path};
use crate::pairing::AppPairing;

/// A send's or a receive's measured progress.
pub const PROGRESS_EVENT: &str = "quicksend-progress";
/// The receiver: what is offered.
pub const INCOMING_EVENT: &str = "quicksend-incoming";
/// How it ended, file by file.
pub const RECEIPT_EVENT: &str = "quicksend-receipt";
/// It could not go on (the connection was lost, or something else).
pub const PROBLEM_EVENT: &str = "quicksend-problem";

/// The most things one send may hold, chosen in the picker.
const MAX_PICKED: usize = 1000;
const TICK: Duration = Duration::from_millis(400);

/// Something chosen in the picker.
struct Picked {
    id: String,
    path: PathBuf,
    folder: bool,
}

/// What the receiver is answering.
enum Answer {
    Accept(Option<Vec<ItemId>>),
    Decline,
}

#[derive(Default)]
pub struct QuickSend {
    picked: Mutex<Vec<Picked>>,
    next_id: Mutex<u64>,
    offer: Mutex<Option<Offer>>,
    folder: Mutex<Option<PathBuf>>,
    live: Mutex<Option<Live>>,
    answer: Mutex<Option<oneshot::Sender<Answer>>>,
    offered: Mutex<Vec<ItemId>>,
}

pub type AppQuickSend = Arc<QuickSend>;

pub fn new_state() -> AppQuickSend {
    Arc::new(QuickSend::default())
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// One chosen thing as Q02 shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PickedView {
    pub id: String,
    pub name: String,
    pub kind: &'static str,
    pub included: bool,
    pub bytes: Option<u64>,
    pub files: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub problem: Option<&'static str>,
}

fn view_of(p: &Picked) -> PickedView {
    let name = p
        .path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let meta = std::fs::metadata(&p.path);
    let (bytes, files, problem) = match (&meta, p.folder) {
        (Err(_), _) => (None, None, Some("missing")),
        (Ok(m), false) => (Some(m.len()), Some(1), None),
        // A folder's size is measured when the offer is made, never guessed here.
        (Ok(_), true) => (None, None, None),
    };
    PickedView {
        id: p.id.clone(),
        name,
        kind: if p.folder { "folder" } else { "file" },
        included: true,
        bytes,
        files,
        problem,
    }
}

/// An offered file as both sides list it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OfferedView {
    pub id: String,
    pub path: String,
    pub size: u64,
}

fn offered(files: Vec<pctwin_quicksend::OfferedFile>) -> Vec<OfferedView> {
    files
        .into_iter()
        .map(|f| OfferedView {
            id: f.id.to_hex(),
            path: f.path,
            size: f.size,
        })
        .collect()
}

/// How one file ended, as the screens show it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum OutcomeView {
    Arrived {
        #[serde(rename = "landedAs")]
        landed_as: String,
    },
    AlreadyThere {
        #[serde(rename = "landedAs")]
        landed_as: String,
    },
    Declined,
    /// It is there but not finished as a proven copy: the words say what to do.
    NeedsAction {
        #[serde(rename = "landedAs")]
        landed_as: String,
        why: String,
    },
    Failed {
        why: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LineView {
    pub path: String,
    pub outcome: OutcomeView,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReceiptView {
    pub side: &'static str,
    pub lines: Vec<LineView>,
    pub stopped: bool,
}

fn receipt_view(side: &'static str, r: &Receipt, stopped: bool) -> ReceiptView {
    ReceiptView {
        side,
        stopped,
        lines: r
            .lines
            .iter()
            .map(|l| LineView {
                path: l.path.clone(),
                outcome: match &l.outcome {
                    Outcome::Arrived { landed_as } => OutcomeView::Arrived {
                        landed_as: landed_as.clone(),
                    },
                    Outcome::AlreadyThere { landed_as } => OutcomeView::AlreadyThere {
                        landed_as: landed_as.clone(),
                    },
                    Outcome::Declined => OutcomeView::Declined,
                    Outcome::NeedsAction { landed_as, why } => OutcomeView::NeedsAction {
                        landed_as: landed_as.clone(),
                        why: why.clone(),
                    },
                    Outcome::Failed { why, .. } => OutcomeView::Failed { why: why.clone() },
                },
            })
            .collect(),
    }
}

/// A send's measured progress (Q12): counted work only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressView {
    pub phase: &'static str,
    pub files_done: u64,
    pub files_total: u64,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub current: Option<String>,
    pub failed: u64,
    pub speed: Option<crate::moving::Range>,
}

fn progress_view(snap: &pctwin_transfer::MeterSnapshot, paused: bool) -> ProgressView {
    let r = &snap.reading;
    let f = &r.files;
    ProgressView {
        phase: if paused || r.paused_here || r.paused_there {
            "paused"
        } else if f.checking > 0 {
            "checking"
        } else {
            "copying"
        },
        files_done: f.copied + f.already_there,
        files_total: f.total,
        bytes_done: r
            .bytes_checked
            .saturating_add(r.bytes_already_there)
            .max(r.bytes_confirmed),
        bytes_total: r.bytes_planned,
        current: None,
        failed: f.failed,
        speed: snap
            .bytes_per_second
            .map(|b| crate::moving::Range { low: b, high: b }),
    }
}

/// What the receiver is offered (Q08).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IncomingView {
    pub files: Vec<OfferedView>,
}

/// Why a Quick Send could not go on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum QsProblem {
    LinkLost { detail: String },
    Failed { detail: String },
}

fn problem_of(e: &pctwin_quicksend::QuickError) -> QsProblem {
    let detail = e.to_string();
    if matches!(
        e,
        pctwin_quicksend::QuickError::Transfer(pctwin_transfer::TransferError::ConnectionDropped)
    ) {
        QsProblem::LinkLost { detail }
    } else {
        QsProblem::Failed { detail }
    }
}

/// Sends the progress until `done`.
fn tick<R: Runtime>(app: AppHandle<R>, live: Live, done: Arc<AtomicBool>) {
    tauri::async_runtime::spawn(async move {
        let mut last = None;
        while !done.load(Ordering::Acquire) {
            let view = progress_view(&live.meter.snapshot(), live.control.paused());
            if last.as_ref() != Some(&view) {
                let _ = app.emit(PROGRESS_EVENT, &view);
                last = Some(view);
            }
            tokio::time::sleep(TICK).await;
        }
    });
}

/* ---------- Commands ---------- */

/// Opens the system's picker for files or folders, and adds what was chosen (Q02).
#[tauri::command]
pub async fn quicksend_pick(
    qs: State<'_, AppQuickSend>,
    folders: bool,
) -> Result<Vec<PickedView>, MoveProblem> {
    let chosen = tauri::async_runtime::spawn_blocking(move || {
        let dialog = rfd::FileDialog::new();
        if folders {
            dialog.pick_folders()
        } else {
            dialog.pick_files()
        }
    })
    .await
    .map_err(|_| MoveProblem::Failed)?
    .unwrap_or_default();
    let mut picked = lock(&qs.picked);
    let mut next = lock(&qs.next_id);
    for path in chosen {
        if picked.len() >= MAX_PICKED || picked.iter().any(|p| p.path == path) {
            continue;
        }
        *next += 1;
        picked.push(Picked {
            id: format!("p{next}"),
            path,
            folder: folders,
        });
    }
    Ok(picked.iter().map(view_of).collect())
}

/// Forgets everything chosen, to start a new send.
#[tauri::command]
pub fn quicksend_clear(qs: State<'_, AppQuickSend>) {
    lock(&qs.picked).clear();
    *lock(&qs.offer) = None;
}

/// The offer of the chosen things the person kept ticked (Q03).
#[tauri::command]
pub async fn quicksend_offer(
    qs: State<'_, AppQuickSend>,
    ids: Vec<String>,
) -> Result<Vec<OfferedView>, MoveProblem> {
    let paths: Vec<PathBuf> = {
        let picked = lock(&qs.picked);
        if ids.iter().any(|id| !picked.iter().any(|p| &p.id == id)) {
            return Err(MoveProblem::Failed);
        }
        picked
            .iter()
            .filter(|p| ids.contains(&p.id))
            .map(|p| p.path.clone())
            .collect()
    };
    let offer = tauri::async_runtime::spawn_blocking(move || Offer::of(&paths))
        .await
        .map_err(|_| MoveProblem::Failed)?
        .map_err(|_| MoveProblem::Failed)?;
    let files = offered(offer.files());
    *lock(&qs.offer) = Some(offer);
    Ok(files)
}

/// The sender: sends the offer over the paired link (Q11 Start).
#[tauri::command]
pub async fn quicksend_send<R: Runtime>(
    app: AppHandle<R>,
    pairing: State<'_, AppPairing>,
    qs: State<'_, AppQuickSend>,
) -> Result<(), MoveProblem> {
    let offer = lock(&qs.offer).take().ok_or(MoveProblem::NotMoving)?;
    let (mut link, _) = pairing.take_link().await.ok_or(MoveProblem::NotPaired)?;
    let live = Live::default();
    *lock(&qs.live) = Some(live.clone());
    let done = Arc::new(AtomicBool::new(false));
    tick(app.clone(), live.clone(), done.clone());
    let runtime = tokio::runtime::Handle::current();
    std::thread::Builder::new()
        .name("pctwin-quicksend".into())
        .spawn(move || {
            let sent = runtime.block_on(pctwin_quicksend::send_live(&mut link, offer, &live));
            done.store(true, Ordering::Release);
            match sent {
                Ok(r) => {
                    let stopped = live.control.stopped();
                    let _ = app.emit(RECEIPT_EVENT, receipt_view("sender", &r, stopped));
                }
                Err(e) => {
                    let _ = app.emit(PROBLEM_EVENT, problem_of(&e));
                }
            }
        })
        .map(|_| ())
        .map_err(|_| MoveProblem::Failed)
}

/// The receiver: waits for the offer over the paired link, shows it (Q08), and receives what the
/// person accepts into the folder they chose.
#[tauri::command]
pub async fn quicksend_receive<R: Runtime>(
    app: AppHandle<R>,
    pairing: State<'_, AppPairing>,
    qs: State<'_, AppQuickSend>,
) -> Result<(), MoveProblem> {
    let (mut link, _) = pairing.take_link().await.ok_or(MoveProblem::NotPaired)?;
    let me = pctwin_scan::list_people()
        .into_iter()
        .find(|p| p.is_me)
        .map(|p| p.account_id)
        .ok_or(MoveProblem::Failed)?;
    let journal_at = journal_path(&app).ok_or(MoveProblem::Failed)?;
    let (tx, rx) = oneshot::channel();
    *lock(&qs.answer) = Some(tx);
    let live = Live::default();
    *lock(&qs.live) = Some(live.clone());
    let qs = Arc::clone(&qs);
    let runtime = tokio::runtime::Handle::current();
    std::thread::Builder::new()
        .name("pctwin-quickreceive".into())
        .spawn(move || {
            let ran = runtime.block_on(async {
                let incoming = pctwin_quicksend::receive_offer(&mut link).await?;
                let files = incoming.files();
                *lock(&qs.offered) = files.iter().map(|f| f.id).collect();
                let _ = app.emit(
                    INCOMING_EVENT,
                    IncomingView {
                        files: offered(files),
                    },
                );
                match rx.await {
                    Ok(Answer::Accept(chosen)) => {
                        let folder = lock(&qs.folder)
                            .clone()
                            .ok_or(pctwin_quicksend::QuickError::NothingChosen)?;
                        if let Some(dir) = journal_at.parent() {
                            let _ = std::fs::create_dir_all(dir);
                        }
                        let journal = pctwin_journal::Journal::open(&journal_at).map_err(|e| {
                            pctwin_quicksend::QuickError::Destination(e.to_string())
                        })?;
                        let done = Arc::new(AtomicBool::new(false));
                        tick(app.clone(), live.clone(), done.clone());
                        let r = incoming
                            .accept_live(
                                &mut link,
                                &folder,
                                chosen.as_deref(),
                                &journal,
                                &me,
                                &live,
                            )
                            .await;
                        done.store(true, Ordering::Release);
                        r.map(Some)
                    }
                    _ => incoming.decline(&mut link).await.map(|()| None),
                }
            });
            match ran {
                Ok(Some(r)) => {
                    let stopped = live.control.stopped();
                    let _ = app.emit(RECEIPT_EVENT, receipt_view("receiver", &r, stopped));
                }
                Ok(None) => {
                    let _ = app.emit(
                        RECEIPT_EVENT,
                        ReceiptView {
                            side: "receiver",
                            lines: Vec::new(),
                            stopped: false,
                        },
                    );
                }
                Err(e) => {
                    let _ = app.emit(PROBLEM_EVENT, problem_of(&e));
                }
            }
        })
        .map(|_| ())
        .map_err(|_| MoveProblem::Failed)
}

/// Where the receiver's files go (Q09): chosen in the system's own picker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderView {
    /// The folder, as the system shows it.
    pub path: String,
    pub free_bytes: Option<u64>,
}

#[tauri::command]
pub async fn quicksend_choose_folder(
    qs: State<'_, AppQuickSend>,
) -> Result<Option<FolderView>, MoveProblem> {
    let chosen = tauri::async_runtime::spawn_blocking(|| rfd::FileDialog::new().pick_folder())
        .await
        .map_err(|_| MoveProblem::Failed)?;
    let Some(folder) = chosen else {
        return Ok(None);
    };
    let free_bytes = pctwin_gate::Destination::open(&folder)
        .ok()
        .and_then(|d| d.available_bytes());
    let view = FolderView {
        path: folder.display().to_string(),
        free_bytes,
    };
    *lock(&qs.folder) = Some(folder);
    Ok(Some(view))
}

/// The receiver's answer (Q08): accept the files `ids` (all of them with none), or decline.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum QsAnswer {
    Accept { ids: Option<Vec<String>> },
    Decline,
}

#[tauri::command]
pub fn quicksend_answer(qs: State<'_, AppQuickSend>, answer: QsAnswer) -> Result<(), MoveProblem> {
    let answer = match answer {
        QsAnswer::Decline => Answer::Decline,
        QsAnswer::Accept { ids } => {
            if lock(&qs.folder).is_none() {
                return Err(MoveProblem::Failed);
            }
            let offered = lock(&qs.offered).clone();
            Answer::Accept(match ids {
                None => None,
                Some(ids) => Some(
                    ids.iter()
                        .map(|id| {
                            ItemId::from_hex(id)
                                .ok()
                                .filter(|i| offered.contains(i))
                                .ok_or(MoveProblem::Failed)
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                ),
            })
        }
    };
    let tx = lock(&qs.answer).take().ok_or(MoveProblem::NotMoving)?;
    let _ = tx.send(answer);
    Ok(())
}

fn with_live(qs: &QuickSend, f: impl FnOnce(&Live)) -> Result<(), MoveProblem> {
    let live = lock(&qs.live);
    f(live.as_ref().ok_or(MoveProblem::NotMoving)?);
    Ok(())
}

#[tauri::command]
pub fn quicksend_pause(qs: State<'_, AppQuickSend>) -> Result<(), MoveProblem> {
    with_live(&qs, |l| l.control.pause())
}

#[tauri::command]
pub fn quicksend_resume(qs: State<'_, AppQuickSend>) -> Result<(), MoveProblem> {
    with_live(&qs, |l| l.control.carry_on())
}

#[tauri::command]
pub fn quicksend_stop(qs: State<'_, AppQuickSend>) -> Result<(), MoveProblem> {
    with_live(&qs, |l| l.control.stop())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_receipt_names_every_outcome_and_needs_action_is_never_arrived() {
        let r = Receipt {
            lines: vec![
                pctwin_quicksend::ReceiptLine {
                    path: "a.txt".into(),
                    outcome: Outcome::Arrived {
                        landed_as: "a.txt".into(),
                    },
                },
                pctwin_quicksend::ReceiptLine {
                    path: "b.txt".into(),
                    outcome: Outcome::NeedsAction {
                        landed_as: "b.txt".into(),
                        why: "open it once".into(),
                    },
                },
                pctwin_quicksend::ReceiptLine {
                    path: "c.txt".into(),
                    outcome: Outcome::Declined,
                },
            ],
        };
        let v = receipt_view("receiver", &r, false);
        assert!(matches!(
            v.lines[1].outcome,
            OutcomeView::NeedsAction { .. }
        ));
        let json = serde_json::to_value(&v).unwrap();
        assert_eq!(json["lines"][0]["outcome"]["kind"], "arrived");
        assert_eq!(json["lines"][1]["outcome"]["kind"], "needs-action");
        assert_eq!(json["lines"][2]["outcome"]["kind"], "declined");
    }

    #[test]
    fn a_missing_pick_says_so_and_a_folder_is_not_sized_by_guess() {
        let dir = tempfile_dir();
        let file = dir.join("a.txt");
        std::fs::write(&file, b"hello").unwrap();
        let p = |path: PathBuf, folder| Picked {
            id: "p1".into(),
            path,
            folder,
        };
        assert_eq!(view_of(&p(file, false)).bytes, Some(5));
        assert_eq!(view_of(&p(dir.clone(), true)).bytes, None);
        assert_eq!(
            view_of(&p(dir.join("gone.txt"), false)).problem,
            Some("missing")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn tempfile_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pctwin-qs-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
