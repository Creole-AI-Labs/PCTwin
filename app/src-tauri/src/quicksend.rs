//! Quick Send for the screens (Q01 to Q14): chosen files and folders to a laptop nearby, over the
//! same pairing, sealed link and safety gate as a move. The screens never name a path: files and
//! folders are chosen in the system's own picker here, and the screens get names, sizes and
//! opaque IDs only. The receiver's files go to its Downloads folder, or to a folder it chose in
//! the system's picker; nothing received is opened. Choosing Receive, typing the one-minute code and
//! matching the number is the receiver's acceptance (Security Design T): once paired, everything
//! offered is received into the folder held open since the pairing page showed it.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use pctwin_move::Live;
use pctwin_quicksend::{Offer, Outcome, Receipt};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

use crate::moving::{MoveProblem, journal_path};
use crate::pairing::AppPairing;

/// A send's or a receive's measured progress.
pub const PROGRESS_EVENT: &str = "quicksend-progress";
/// The receiver: what is being received (names and sizes, for the progress and Done pages).
pub const INCOMING_EVENT: &str = "quicksend-incoming";
/// How it ended, file by file.
pub const RECEIPT_EVENT: &str = "quicksend-receipt";
/// It could not go on (the connection was lost, or something else).
pub const PROBLEM_EVENT: &str = "quicksend-problem";
/// Things dropped on the window while the pick page is open: the whole chosen list.
pub const PICKED_EVENT: &str = "quicksend-picked";
/// Whether something is being dragged over the window while the pick page is open.
pub const DRAGGING_EVENT: &str = "quicksend-dragging";

/// The most things one send may hold, chosen in the picker.
const MAX_PICKED: usize = 1000;
const TICK: Duration = Duration::from_millis(400);

/// Something chosen in the picker.
struct Picked {
    id: String,
    path: PathBuf,
    folder: bool,
}

/// The folder a receive writes into: opened when the pairing page showed it (the very folder
/// written to, never whatever is at its path later), and the path it was opened at.
struct Held {
    path: PathBuf,
    dest: pctwin_gate::Destination,
}

#[derive(Default)]
pub struct QuickSend {
    /// The pick page is open: things dropped on the window are added.
    accepting: AtomicBool,
    picked: Mutex<Vec<Picked>>,
    next_id: Mutex<u64>,
    offer: Mutex<Option<Offer>>,
    /// The receiver's folder, held open from when the pairing page showed it; taken by the receive
    /// it was shown for, so each receive holds its folder afresh.
    folder: Mutex<Option<Held>>,
    /// The folder the person chose this session (only its path): the next receive opens it again
    /// rather than trusting an old handle. Without one, receives go to the system's Downloads folder.
    remembered: Mutex<Option<PathBuf>>,
    /// Where the last receive saved its files: the only folder Open folder opens.
    received_into: Mutex<Option<PathBuf>>,
    live: Mutex<Option<Live>>,
    /// What the offer being made was made of (the chosen things), for sending the missing pieces.
    chosen: Mutex<Vec<PathBuf>>,
    /// The last send from this window: what was chosen, its offer's ID and its receipt (Q19).
    last: Mutex<Option<LastSend>>,
}

/// A send that ended, as Q19 needs it: the same chosen things, offered again under the same
/// offer ID, keep their file IDs, so the receipt says which of them are missing.
struct LastSend {
    chosen: Vec<PathBuf>,
    laptop: pctwin_record::LaptopId,
    receipt: Receipt,
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
            path: shown_safely(&f.path),
            size: f.size,
        })
        .collect()
}

/// A name or path as the screens may show it: characters that reorder or hide text (direction
/// overrides, isolates and marks) and control characters are shown as their code ("[U+202E]"), so
/// a name from another laptop can never look like something else ("invoice\u{202E}fdp.exe" would
/// show as "invoiceexe.pdf"). The file keeps its real name; only what is shown changes.
fn shown_safely(name: &str) -> String {
    let hidden = |c: char| {
        c.is_control()
            || matches!(c, '\u{061C}' | '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
    };
    let mut out = String::with_capacity(name.len());
    for c in name.chars() {
        if hidden(c) {
            out.push_str(&format!("[U+{:04X}]", c as u32));
        } else {
            out.push(c);
        }
    }
    out
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
        /// The screens' own words for it, when there are some (the engine's are for More details).
        #[serde(skip_serializing_if = "Option::is_none")]
        word: Option<&'static str>,
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
                path: shown_safely(&l.path),
                outcome: match &l.outcome {
                    Outcome::Arrived { landed_as } => OutcomeView::Arrived {
                        landed_as: shown_safely(landed_as),
                    },
                    Outcome::AlreadyThere { landed_as } => OutcomeView::AlreadyThere {
                        landed_as: shown_safely(landed_as),
                    },
                    Outcome::Declined => OutcomeView::Declined,
                    Outcome::NeedsAction { landed_as, why } => OutcomeView::NeedsAction {
                        landed_as: shown_safely(landed_as),
                        why: why.clone(),
                        word: (why == pctwin_quicksend::MARK_NOT_KEPT).then_some("mark-not-kept"),
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

/// Adds `paths` to what is chosen (each once, at most [`MAX_PICKED`]), and returns the list.
fn add(qs: &QuickSend, paths: Vec<PathBuf>) -> Vec<PickedView> {
    let mut picked = lock(&qs.picked);
    let mut next = lock(&qs.next_id);
    for path in paths {
        if picked.len() >= MAX_PICKED || picked.iter().any(|p| p.path == path) {
            continue;
        }
        *next += 1;
        let folder = path.is_dir();
        picked.push(Picked {
            id: format!("p{next}"),
            path,
            folder,
        });
    }
    picked.iter().map(view_of).collect()
}

/// Things dropped on the window: added only while the pick page is open (so a drop anywhere else
/// does nothing), and the screens are sent the list (names and sizes, never a path).
pub fn dropped<R: Runtime>(app: &AppHandle<R>, paths: Vec<PathBuf>) {
    use tauri::Manager;
    let qs = app.state::<AppQuickSend>();
    if !qs.accepting.load(Ordering::Acquire) {
        return;
    }
    let list = add(&qs, paths);
    let _ = app.emit(PICKED_EVENT, list);
}

/// Something is (or no longer is) dragged over the window: told only while the pick page is open.
pub fn dragging<R: Runtime>(app: &AppHandle<R>, over: bool) {
    use tauri::Manager;
    if app
        .state::<AppQuickSend>()
        .accepting
        .load(Ordering::Acquire)
    {
        let _ = app.emit(DRAGGING_EVENT, over);
    }
}

/// The pick page opened (`on`) or closed: drops are taken only while it is open.
#[tauri::command]
pub fn quicksend_accept_drops(qs: State<'_, AppQuickSend>, on: bool) {
    qs.accepting.store(on, Ordering::Release);
}

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
    Ok(add(&qs, chosen))
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
    *lock(&qs.chosen) = paths.clone();
    let offer = tauri::async_runtime::spawn_blocking(move || Offer::of(&paths))
        .await
        .map_err(|_| MoveProblem::Failed)?
        .map_err(|e| offer_problem(&e))?;
    let files = offered(offer.files());
    *lock(&qs.offer) = Some(offer);
    Ok(files)
}

/// The sender, after a send that did not bring everything (Q19): the same chosen things offered
/// again under the same offer ID, keeping only what the last receipt says did not arrive. What
/// changed since is offered as it is now, and checked and accepted again. Nothing to send again
/// is an empty list; no earlier send here is `NotMoving`.
#[tauri::command]
pub async fn quicksend_offer_missing(
    qs: State<'_, AppQuickSend>,
) -> Result<Vec<OfferedView>, MoveProblem> {
    let (chosen, laptop, receipt) = {
        let last = lock(&qs.last);
        let last = last.as_ref().ok_or(MoveProblem::NotMoving)?;
        (last.chosen.clone(), last.laptop, last.receipt.clone())
    };
    *lock(&qs.chosen) = chosen.clone();
    let offer = tauri::async_runtime::spawn_blocking(move || {
        Offer::of_again(&chosen, laptop).map(|o| o.only_missing(&receipt))
    })
    .await
    .map_err(|_| MoveProblem::Failed)?
    .map_err(|e| offer_problem(&e))?;
    let files = offered(offer.files());
    *lock(&qs.offer) = Some(offer);
    Ok(files)
}

/// Why an offer could not be made, as the screens say it: too many files is said as that, before
/// anything is sent.
fn offer_problem(e: &pctwin_quicksend::QuickError) -> MoveProblem {
    match e {
        pctwin_quicksend::QuickError::TooMany { .. } => MoveProblem::TooManyFiles,
        _ => MoveProblem::Failed,
    }
}

/// The sender: sends the offer over the paired link (Q11 Start).
#[tauri::command]
pub async fn quicksend_send<R: Runtime>(
    app: AppHandle<R>,
    pairing: State<'_, AppPairing>,
    qs: State<'_, AppQuickSend>,
) -> Result<(), MoveProblem> {
    let offer = lock(&qs.offer).take().ok_or(MoveProblem::NotMoving)?;
    let laptop = offer.laptop();
    let chosen = lock(&qs.chosen).clone();
    let state = qs.inner().clone();
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
                    let view = receipt_view("sender", &r, stopped);
                    *lock(&state.last) = Some(LastSend {
                        chosen,
                        laptop,
                        receipt: r.clone(),
                    });
                    crate::history::add(
                        &app,
                        "send",
                        None,
                        crate::history::send_state(&view),
                        0,
                        &crate::history::receipt_kept(&view),
                    );
                    let _ = app.emit(RECEIPT_EVENT, view);
                }
                Err(e) => {
                    // No receipt for this send: the missing pieces of an earlier one are never
                    // offered as if they were this one's.
                    lock(&state.last).take();
                    let _ = app.emit(PROBLEM_EVENT, problem_of(&e));
                }
            }
        })
        .map(|_| ())
        .map_err(|_| MoveProblem::Failed)
}

/// The receiver: once paired, receives everything offered over the paired link into the folder
/// held for this receive. There is no second question: choosing Receive, typing the code and
/// matching the number was the acceptance (Security Design T). With no folder held, nothing is
/// taken from the link and the screens say so.
#[tauri::command]
pub async fn quicksend_receive<R: Runtime>(
    app: AppHandle<R>,
    pairing: State<'_, AppPairing>,
    qs: State<'_, AppQuickSend>,
) -> Result<(), MoveProblem> {
    let me = pctwin_scan::list_people()
        .into_iter()
        .find(|p| p.is_me)
        .map(|p| p.account_id)
        .ok_or(MoveProblem::Failed)?;
    let journal_at = journal_path(&app).ok_or(MoveProblem::Failed)?;
    // Held for this receive only: the next one holds its folder again.
    let held = lock(&qs.folder).take().ok_or(MoveProblem::Failed)?;
    let (mut link, _) = pairing.take_link().await.ok_or(MoveProblem::NotPaired)?;
    *lock(&qs.received_into) = Some(held.path.clone());
    let live = Live::default();
    *lock(&qs.live) = Some(live.clone());
    let runtime = tokio::runtime::Handle::current();
    std::thread::Builder::new()
        .name("pctwin-quickreceive".into())
        .spawn(move || {
            let ran = runtime.block_on(async {
                let incoming = pctwin_quicksend::receive_offer(&mut link).await?;
                let _ = app.emit(
                    INCOMING_EVENT,
                    IncomingView {
                        files: offered(incoming.files()),
                    },
                );
                if let Some(dir) = journal_at.parent() {
                    let _ = std::fs::create_dir_all(dir);
                }
                let journal = pctwin_journal::Journal::open(&journal_at)
                    .map_err(|e| pctwin_quicksend::QuickError::Destination(e.to_string()))?;
                let done = Arc::new(AtomicBool::new(false));
                tick(app.clone(), live.clone(), done.clone());
                // Everything offered (`None`), into the held folder.
                let r = incoming
                    .accept_into(&mut link, held.dest, None, &journal, &me, &live)
                    .await;
                done.store(true, Ordering::Release);
                r
            });
            match ran {
                Ok(r) => {
                    let stopped = live.control.stopped();
                    let view = receipt_view("receiver", &r, stopped);
                    crate::history::add(
                        &app,
                        "receive",
                        None,
                        crate::history::send_state(&view),
                        0,
                        &crate::history::receipt_kept(&view),
                    );
                    let _ = app.emit(RECEIPT_EVENT, view);
                }
                Err(e) => {
                    let _ = app.emit(PROBLEM_EVENT, problem_of(&e));
                }
            }
        })
        .map(|_| ())
        .map_err(|_| MoveProblem::Failed)
}

/// Where the receiver's files go, as the pairing page says it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderView {
    /// The folder's own name ("Downloads"), shown safely.
    pub name: String,
    /// The whole folder, as the system shows it (for More details).
    pub path: String,
    pub free_bytes: Option<u64>,
}

/// The folder the next receive saves into, held open from now: the one the person chose this
/// session, opened again at its path, or else the system's Downloads folder. A chosen folder
/// that cannot be opened any more (its drive was taken out) is forgotten for Downloads. Neither
/// one there is a problem, and the screens ask the person to choose a folder.
#[tauri::command]
pub fn quicksend_folder<R: Runtime>(
    app: AppHandle<R>,
    qs: State<'_, AppQuickSend>,
) -> Result<FolderView, MoveProblem> {
    let remembered = lock(&qs.remembered).clone();
    if let Some(chosen) = remembered {
        if let Some(view) = held_folder(&qs, &chosen) {
            return Ok(view);
        }
        lock(&qs.remembered).take();
    }
    let downloads = app.path().download_dir().map_err(|_| MoveProblem::Failed)?;
    held_folder(&qs, &downloads).ok_or(MoveProblem::Failed)
}

/// Change: a folder chosen in the system's own picker, held for the next receive in place of the
/// one shown, and remembered (its path) for the rest of the session. `None` when none was chosen.
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
    let view = held_folder(&qs, &folder).ok_or(MoveProblem::Failed)?;
    *lock(&qs.remembered) = Some(folder);
    Ok(Some(view))
}

/// Opens the folder at `folder` and holds it for the next receive, replacing any held before;
/// what the screens show of it. `None` when it cannot be opened.
fn held_folder(qs: &QuickSend, folder: &std::path::Path) -> Option<FolderView> {
    let dest = pctwin_gate::Destination::open(folder).ok()?;
    let name = folder.file_name().map_or_else(
        || folder.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    let view = FolderView {
        name: shown_safely(&name),
        path: shown_safely(&folder.display().to_string()),
        free_bytes: dest.available_bytes(),
    };
    *lock(&qs.folder) = Some(Held {
        path: folder.to_path_buf(),
        dest,
    });
    Some(view)
}

/// Open folder, on the Done page: shows the folder the last receive saved into in the system's
/// file manager. Only that folder, and only while it is a folder: never a received file, and
/// never a path from the screens.
#[tauri::command]
pub fn quicksend_open_folder<R: Runtime>(
    app: AppHandle<R>,
    qs: State<'_, AppQuickSend>,
) -> Result<(), MoveProblem> {
    use tauri_plugin_opener::OpenerExt;
    let folder = lock(&qs.received_into)
        .clone()
        .ok_or(MoveProblem::NotMoving)?;
    if !std::fs::symlink_metadata(&folder).is_ok_and(|m| m.is_dir()) {
        return Err(MoveProblem::Failed);
    }
    app.opener()
        .open_path(folder.to_string_lossy(), None::<&str>)
        .map_err(|_| MoveProblem::Failed)
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
    use pctwin_record::ItemId;

    #[test]
    fn a_receipt_names_every_outcome_and_needs_action_is_never_arrived() {
        let r = Receipt {
            lines: vec![
                pctwin_quicksend::ReceiptLine {
                    id: ItemId::from_hex(&"0".repeat(32)).unwrap(),
                    path: "a.txt".into(),
                    outcome: Outcome::Arrived {
                        landed_as: "a.txt".into(),
                    },
                },
                pctwin_quicksend::ReceiptLine {
                    id: ItemId::from_hex(&"0".repeat(32)).unwrap(),
                    path: "b.txt".into(),
                    outcome: Outcome::NeedsAction {
                        landed_as: "b.txt".into(),
                        why: "open it once".into(),
                    },
                },
                pctwin_quicksend::ReceiptLine {
                    id: ItemId::from_hex(&"0".repeat(32)).unwrap(),
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
        assert!(json["lines"][1]["outcome"].get("word").is_none());
        // A stranger's file whose mark the drive could not keep: the screens' own words.
        let unmarked = Receipt {
            lines: vec![pctwin_quicksend::ReceiptLine {
                id: ItemId::from_hex(&"0".repeat(32)).unwrap(),
                path: "a.exe".into(),
                outcome: Outcome::NeedsAction {
                    landed_as: "a.exe".into(),
                    why: pctwin_quicksend::MARK_NOT_KEPT.into(),
                },
            }],
        };
        let u = serde_json::to_value(receipt_view("receiver", &unmarked, false)).unwrap();
        assert_eq!(u["lines"][0]["outcome"]["word"], "mark-not-kept");
        assert_eq!(json["lines"][0]["outcome"]["kind"], "arrived");
        assert_eq!(json["lines"][1]["outcome"]["kind"], "needs-action");
        assert_eq!(json["lines"][2]["outcome"]["kind"], "declined");
    }

    /// Exit-gate review (24 October 2026): a name that reorders itself when shown
    /// (`invoice\u{202E}fdp.exe` shows as "invoiceexe.pdf") or hides control characters is shown
    /// with them as their codes, on the offer and on the receipt; plain names are unchanged.
    #[test]
    fn a_name_that_reorders_or_hides_text_is_shown_with_its_codes() {
        assert_eq!(
            shown_safely("Docs/invoice\u{202E}fdp.exe"),
            "Docs/invoice[U+202E]fdp.exe"
        );
        assert_eq!(
            shown_safely("a\u{2066}b\u{0007}c\u{0085}"),
            "a[U+2066]b[U+0007]c[U+0085]"
        );
        assert_eq!(shown_safely("Holiday/été 2.jpg"), "Holiday/été 2.jpg");
        let files = offered(vec![pctwin_quicksend::OfferedFile {
            id: ItemId::from_hex(&"0".repeat(32)).unwrap(),
            path: "invoice\u{202E}fdp.exe".into(),
            size: 1,
        }]);
        assert_eq!(files[0].path, "invoice[U+202E]fdp.exe");
        let r = Receipt {
            lines: vec![pctwin_quicksend::ReceiptLine {
                id: ItemId::from_hex(&"0".repeat(32)).unwrap(),
                path: "invoice\u{202E}fdp.exe".into(),
                outcome: Outcome::Arrived {
                    landed_as: "invoice\u{202E}fdp.exe".into(),
                },
            }],
        };
        let v = receipt_view("receiver", &r, false);
        assert_eq!(v.lines[0].path, "invoice[U+202E]fdp.exe");
        assert!(
            matches!(&v.lines[0].outcome, OutcomeView::Arrived { landed_as } if landed_as == "invoice[U+202E]fdp.exe")
        );
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

    /// Exit-gate review (24 October 2026), and 26 October 2026: the folder is held open from when
    /// it is shown, so a folder put at its path afterwards is not the one written to; a receive
    /// takes it, so the next receive holds its folder again (by the remembered path, never the
    /// old handle).
    #[test]
    fn the_folder_is_held_from_when_it_is_shown_and_held_again_for_each_receive() {
        let qs = QuickSend::default();
        let dir = tempfile_dir().join("held");
        let chosen = dir.join("Inbox");
        std::fs::create_dir_all(&chosen).unwrap();
        let view = held_folder(&qs, &chosen).unwrap();
        assert_eq!(view.name, "Inbox");
        // Swapped after it was shown: the path now names another folder.
        let moved = dir.join("Inbox-moved");
        let swapped = std::fs::rename(&chosen, &moved).is_ok();
        if swapped {
            std::fs::create_dir_all(&chosen).unwrap();
            let held = lock(&qs.folder).take().unwrap();
            assert_eq!(held.path, chosen);
            let id = held.dest.folder_identity("").unwrap();
            let there = pctwin_gate::Destination::open(&moved)
                .unwrap()
                .folder_identity("")
                .unwrap();
            assert_eq!(
                id, there,
                "the held folder is the one shown, wherever it is now"
            );
            // The next receive holds the folder at the path again: today's folder there.
            held_folder(&qs, &chosen).unwrap();
            let again = lock(&qs.folder).take().unwrap();
            let now = pctwin_gate::Destination::open(&chosen)
                .unwrap()
                .folder_identity("")
                .unwrap();
            assert_eq!(again.dest.folder_identity("").unwrap(), now);
            assert_ne!(now, id, "never the old handle");
        } else {
            // Windows keeps a folder held open from being renamed: it cannot be swapped at all.
            lock(&qs.folder).take();
        }
        assert!(lock(&qs.folder).is_none(), "taken by the receive");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A folder that cannot be opened is never held.
    #[test]
    fn a_folder_that_cannot_be_opened_is_not_held() {
        let qs = QuickSend::default();
        let dir = tempfile_dir().join("never-made");
        assert!(held_folder(&qs, &dir.join("nope")).is_none());
        assert!(lock(&qs.folder).is_none());
    }

    #[test]
    fn an_offer_of_too_many_files_is_said_as_that() {
        assert_eq!(
            offer_problem(&pctwin_quicksend::QuickError::TooMany { files: 2_000_000 }),
            MoveProblem::TooManyFiles
        );
        assert_eq!(
            offer_problem(&pctwin_quicksend::QuickError::NothingChosen),
            MoveProblem::Failed
        );
    }

    fn tempfile_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pctwin-qs-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
