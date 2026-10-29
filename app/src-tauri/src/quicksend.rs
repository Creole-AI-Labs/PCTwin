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
use pctwin_gate::FileId;

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
/// The receiver: a note or link arrived (shown with Copy; a link opens only by the person).
pub const NOTE_EVENT: &str = "quicksend-note";
/// The sender: the other laptop took the note (it arrived and passed its checks).
pub const NOTE_SENT_EVENT: &str = "quicksend-note-sent";
/// After an exchange over the six-digit pairing: "Remember this connection" may be offered.
pub const REMEMBER_EVENT: &str = "quicksend-remember";
/// Files or folders the system handed PCTwin (Send to, a file manager entry, the share action,
/// a drop on the widget), added to what is chosen: open the pick page.
pub const LAUNCH_EVENT: &str = "quicksend-launch";
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
pub(crate) struct Held {
    pub(crate) path: PathBuf,
    pub(crate) identity: FileId,
    pub(crate) dest: pctwin_gate::Destination,
}

/// A folder known by its path and by what it is (its identity when it was first held), so that a
/// folder put at that path later, or a link put there, is never taken for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Known {
    pub(crate) path: PathBuf,
    pub(crate) identity: FileId,
}

#[derive(Default)]
pub struct QuickSend {
    /// The pick page is open: things dropped on the window are added.
    accepting: AtomicBool,
    picked: Mutex<Vec<Picked>>,
    next_id: Mutex<u64>,
    pub(crate) offer: Mutex<Option<Offer>>,
    /// The receiver's folder, held open from when the pairing page showed it; taken by the receive
    /// it was shown for, so each receive holds its folder afresh.
    pub(crate) folder: Mutex<Option<Held>>,
    /// The folder the person chose this session: the next receive opens it again at its path
    /// rather than trusting an old handle, and only if it is still that folder. Without one,
    /// receives go to the system's Downloads folder.
    pub(crate) remembered: Mutex<Option<Known>>,
    /// What the Downloads folder was when first held this session.
    downloads: Mutex<Option<FileId>>,
    /// Where the last receive saved its files: the only folder Open folder opens.
    pub(crate) received_into: Mutex<Option<Known>>,
    pub(crate) live: Mutex<Option<Live>>,
    /// What the offer being made was made of (the chosen things), for sending the missing pieces.
    pub(crate) chosen: Mutex<Vec<PathBuf>>,
    /// The last send from this window: what was chosen, its offer's ID and its receipt (Q19).
    last: Mutex<Option<LastSend>>,
    /// The files the last receive wrote, given the arrival time, with their original dates: what
    /// Keep original dates gives back. Kept in memory for this window only, never on disk.
    pub(crate) dated: Mutex<Vec<Dated>>,
    /// A receive waiting for the person to say where the files go (decided 29 October 2026: the
    /// receiver is asked when they arrive, not before): [`quicksend_save_here`] answers it.
    save: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    /// The last note or link received: the only web address Open may open.
    pub(crate) last_note: Mutex<Option<pctwin_quicksend::Note>>,
}

/// A file a receive wrote and gave the arrival time, as Keep original dates needs it: where it
/// is in the folder, which file it is, and the date it came with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Dated {
    stored: String,
    identity: FileId,
    original: std::time::SystemTime,
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

pub(crate) fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
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

pub(crate) fn offered(files: Vec<pctwin_quicksend::OfferedFile>) -> Vec<OfferedView> {
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
pub(crate) fn shown_safely(name: &str) -> String {
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
    /// The receiver: how many arrived files now show the arrival time, so Keep original dates
    /// can give theirs back. Always 0 on the sender.
    #[serde(rename = "newDates")]
    pub new_dates: u64,
}

pub(crate) fn receipt_view(side: &'static str, r: &Receipt, stopped: bool) -> ReceiptView {
    ReceiptView {
        side,
        stopped,
        new_dates: 0,
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
    /// The measured time left in whole minutes, as the full move works it out; `None` while it
    /// is still being worked out.
    pub minutes_left: Option<crate::moving::Range>,
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
        bytes_done: crate::moving::bytes_shown(r),
        bytes_total: r.bytes_planned,
        current: None,
        failed: f.failed,
        speed: snap
            .bytes_per_second
            .map(|b| crate::moving::Range { low: b, high: b }),
        minutes_left: match snap.time_left {
            pctwin_transfer::TimeLeft::About { seconds } => {
                let m = seconds.div_ceil(60);
                Some(crate::moving::Range { low: m, high: m })
            }
            _ => None,
        },
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
    LinkLost {
        detail: String,
    },
    Failed {
        detail: String,
    },
    /// The receiving folder has no room for the offer: it was declined and nothing was written.
    NoRoom {
        needed: u64,
        room: u64,
    },
}

pub(crate) fn problem_of(e: &pctwin_quicksend::QuickError) -> QsProblem {
    if let pctwin_quicksend::QuickError::NoRoom { needed, room } = e {
        return QsProblem::NoRoom {
            needed: *needed,
            room: *room,
        };
    }
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
pub(crate) fn tick<R: Runtime>(app: AppHandle<R>, live: Live, done: Arc<AtomicBool>) {
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
pub(crate) fn add(qs: &QuickSend, paths: Vec<PathBuf>) -> Vec<PickedView> {
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

/// Things dropped on the window, or handed over by the system (Send to, a file manager entry,
/// the share action): added to what is chosen, and the screens are sent the list (names and
/// sizes, never a path): while the pick page is open as the pick list, otherwise as a launch the
/// screens may open the pick page for. Nothing is sent before the person chooses Send.
pub fn dropped<R: Runtime>(app: &AppHandle<R>, paths: Vec<PathBuf>) {
    use tauri::Manager;
    let qs = app.state::<AppQuickSend>();
    let list = add(&qs, paths);
    if qs.accepting.load(Ordering::Acquire) {
        let _ = app.emit(PICKED_EVENT, list);
    } else {
        // A drop on the widget, or files the system handed PCTwin: the screens open the pick
        // page with them, or ignore it (nothing is sent before the person chooses Send).
        let _ = app.emit(LAUNCH_EVENT, list);
    }
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
    let known = app
        .state::<crate::remembered::AppRemembered>()
        .inner()
        .clone();
    let live = Live::default();
    *lock(&qs.live) = Some(live.clone());
    let done = Arc::new(AtomicBool::new(false));
    tick(app.clone(), live.clone(), done.clone());
    let runtime = tokio::runtime::Handle::current();
    std::thread::Builder::new()
        .name("pctwin-quicksend".into())
        .spawn(move || {
            let sent = runtime.block_on(async {
                let pending = crate::remembered::swap_keys(&mut link).await?;
                let r = pctwin_quicksend::send_live(&mut link, offer, &live).await?;
                known.offer_remember(&app, pending);
                Ok::<_, pctwin_quicksend::QuickError>(r)
            });
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

/// The receiver: once paired, says what is offered and asks where it goes (Security Design T,
/// 29 October 2026: choosing Receive, typing the code and matching the number accepts the send;
/// where it goes is asked when it arrives). Downloads, or the folder chosen this session, is the
/// one shown already; "Choose another folder" changes it ([`quicksend_choose_folder`]); Save
/// ([`quicksend_save_here`]) receives into whichever is held then. Nothing is taken from the
/// link before that, and never with no folder held.
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
    let (mut link, _) = pairing.take_link().await.ok_or(MoveProblem::NotPaired)?;
    let known = app
        .state::<crate::remembered::AppRemembered>()
        .inner()
        .clone();
    lock(&qs.dated).clear();
    let (answered, answer) = tokio::sync::oneshot::channel();
    *lock(&qs.save) = Some(answered);
    let state = Arc::clone(&qs);
    let live = Live::default();
    *lock(&qs.live) = Some(live.clone());
    let runtime = tokio::runtime::Handle::current();
    std::thread::Builder::new()
        .name("pctwin-quickreceive".into())
        .spawn(move || {
            let ran = runtime.block_on(async {
                let pending = crate::remembered::swap_keys(&mut link).await?;
                let incoming = match pctwin_quicksend::receive(&mut link).await? {
                    pctwin_quicksend::Arrival::Files(incoming) => incoming,
                    pctwin_quicksend::Arrival::Note(note) => {
                        lock(&state.save).take();
                        show_note(&app, &state, note, None);
                        known.offer_remember(&app, pending);
                        return Ok(None);
                    }
                };
                let _ = app.emit(
                    INCOMING_EVENT,
                    IncomingView {
                        files: offered(incoming.files()),
                    },
                );
                // Where it goes: asked now, answered by Save (the sender waits as long).
                wait_for_save(
                    answer,
                    live.control.wait_stopped(),
                    pctwin_quicksend::ANSWER_WAIT,
                )
                .await?;
                // Held for this receive only: the next one holds its folder again.
                let held = take_held(&state).map_err(|_| {
                    pctwin_quicksend::QuickError::Destination("no folder was chosen".into())
                })?;
                let into = Known {
                    path: held.path.clone(),
                    identity: held.identity,
                };
                *lock(&state.received_into) = Some(into);
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
                lock(&state.save).take();
                done.store(true, Ordering::Release);
                if r.is_ok() {
                    known.offer_remember(&app, pending);
                }
                r.map(Some)
            });
            match ran {
                Ok(None) => {}
                Ok(Some(r)) => {
                    let stopped = live.control.stopped();
                    let mut view = receipt_view("receiver", &r, stopped);
                    // Received files show when they arrived, at the top of the folder; the person
                    // can give them their original dates back from the Done page.
                    let into = lock(&state.received_into).clone();
                    let dated = into
                        .as_ref()
                        .and_then(reopened)
                        .map(|dest| arrival_dates(&dest, &r, std::time::SystemTime::now()))
                        .unwrap_or_default();
                    view.new_dates = dated.len() as u64;
                    *lock(&state.dated) = dated;
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

/// A note or link as the receiving screen shows it: the text exactly as sent (checked to hold
/// nothing that hides or reorders text), whether it is a link and the site it leads to, and the
/// remembered connection it came from (`None`: the six-digit pairing).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NoteView {
    pub text: String,
    pub kind: &'static str,
    pub host: Option<String>,
    pub from: Option<String>,
}

pub(crate) fn note_view(note: &pctwin_quicksend::Note, from: Option<String>) -> NoteView {
    let (kind, host) = match note.kind() {
        pctwin_quicksend::NoteKind::Text => ("text", None),
        pctwin_quicksend::NoteKind::Link { host } => ("link", Some(host.clone())),
    };
    NoteView {
        text: note.text().to_string(),
        kind,
        host,
        from,
    }
}

/// Shows a note that arrived and keeps it, so Open can open its link when the person asks.
pub(crate) fn show_note<R: Runtime>(
    app: &AppHandle<R>,
    qs: &QuickSend,
    note: pctwin_quicksend::Note,
    from: Option<String>,
) {
    let _ = app.emit(NOTE_EVENT, note_view(&note, from));
    *lock(&qs.last_note) = Some(note);
}

/// The other laptop took the note: it arrived and passed its checks (never that it was read).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NoteSentView {
    pub taken: bool,
}

/// The sender: sends a note or link over the paired link (typed lines or one web address). The
/// note is checked before anything is sent; `quicksend-note-sent` says the other laptop took it.
#[tauri::command]
pub async fn quicksend_send_note<R: Runtime>(
    app: AppHandle<R>,
    pairing: State<'_, AppPairing>,
    text: String,
) -> Result<(), crate::remembered::QsRefusal> {
    let note = pctwin_quicksend::Note::new(&text).map_err(crate::remembered::note_refusal)?;
    let (mut link, _) = pairing
        .take_link()
        .await
        .ok_or(crate::remembered::QsRefusal::NotPaired)?;
    let known = app
        .state::<crate::remembered::AppRemembered>()
        .inner()
        .clone();
    tauri::async_runtime::spawn(async move {
        let sent = async {
            let pending = crate::remembered::swap_keys(&mut link).await?;
            pctwin_quicksend::send_note(&mut link, &note).await?;
            known.offer_remember(&app, pending);
            Ok::<_, pctwin_quicksend::QuickError>(())
        }
        .await;
        match sent {
            Ok(()) => {
                let _ = app.emit(NOTE_SENT_EVENT, NoteSentView { taken: true });
            }
            Err(e) => {
                let _ = app.emit(PROBLEM_EVENT, problem_of(&e));
            }
        }
    });
    Ok(())
}

/// Open, on a received link: the system's browser opens the last received note's web address,
/// only when it is a link (http or https, no user name), and only because the person chose Open.
/// The screens never name the address.
#[tauri::command]
pub fn quicksend_open_link<R: Runtime>(
    app: AppHandle<R>,
    qs: State<'_, AppQuickSend>,
) -> Result<(), crate::remembered::QsRefusal> {
    use tauri_plugin_opener::OpenerExt;
    let link = link_to_open(&qs)?;
    app.opener()
        .open_url(link, None::<&str>)
        .map_err(|_| crate::remembered::QsRefusal::Failed)
}

fn link_to_open(qs: &QuickSend) -> Result<String, crate::remembered::QsRefusal> {
    lock(&qs.last_note)
        .as_ref()
        .and_then(|n| n.link().map(str::to_string))
        .ok_or(crate::remembered::QsRefusal::NoLink)
}

/// Files and folders the system handed PCTwin when it started (Windows "Send to", a Linux file
/// manager entry, the Mac open action: `--send` and the paths after it), added to what is
/// chosen. Asked by the pick page; the list is the whole chosen list.
#[tauri::command]
pub fn quicksend_launch_files(qs: State<'_, AppQuickSend>) -> Vec<PickedView> {
    add(&qs, crate::handed::take())
}

/// The folder a receive saved into, opened again, only while it is still that very folder.
pub(crate) fn reopened(known: &Known) -> Option<pctwin_gate::Destination> {
    open_plain(&known.path)
        .filter(|(_, identity)| *identity == known.identity)
        .map(|(dest, _)| dest)
}

/// Gives each file this receive wrote (arrived, not one already there, which is the person's own)
/// the time `now`, and says which ones and the date each came with. A file that cannot be read or
/// changed keeps its date and is left out.
pub(crate) fn arrival_dates(
    dest: &pctwin_gate::Destination,
    r: &Receipt,
    now: std::time::SystemTime,
) -> Vec<Dated> {
    r.lines
        .iter()
        .filter_map(|l| match &l.outcome {
            Outcome::Arrived { landed_as } => Some(landed_as),
            _ => None,
        })
        .filter_map(|stored| {
            let stat = dest.stat(stored).ok()??;
            let original = stat.modified?;
            dest.set_modified_time(stored, stat.id, now)
                .ok()?
                .then(|| Dated {
                    stored: stored.clone(),
                    identity: stat.id,
                    original,
                })
        })
        .collect()
}

/// Keep original dates, on the receiver's Done page: the files the last receive wrote get back
/// the dates they came with. Returns how many did; a file moved, changed for another or gone since
/// keeps its date. Once only: the dates are then forgotten.
#[tauri::command]
pub fn quicksend_keep_original_dates(qs: State<'_, AppQuickSend>) -> Result<u64, MoveProblem> {
    keep_original_dates(&qs)
}

fn keep_original_dates(qs: &QuickSend) -> Result<u64, MoveProblem> {
    let dated = std::mem::take(&mut *lock(&qs.dated));
    if dated.is_empty() {
        return Err(MoveProblem::NotMoving);
    }
    let known = lock(&qs.received_into)
        .clone()
        .ok_or(MoveProblem::NotMoving)?;
    let dest = reopened(&known).ok_or(MoveProblem::Failed)?;
    Ok(dated
        .iter()
        .filter(|d| {
            dest.set_modified_time(&d.stored, d.identity, d.original)
                .unwrap_or(false)
        })
        .count() as u64)
}

/// Save: the files that arrived go into the folder held now (Downloads, or the one chosen).
#[tauri::command]
pub fn quicksend_save_here(qs: State<'_, AppQuickSend>) -> Result<(), MoveProblem> {
    if lock(&qs.folder).is_none() {
        return Err(MoveProblem::Failed);
    }
    let waiting = lock(&qs.save).take().ok_or(MoveProblem::NotMoving)?;
    waiting.send(()).map_err(|_| MoveProblem::NotMoving)
}

/// Waits for the person's Save, unless they stop or the sender's own wait for an answer is over.
async fn wait_for_save(
    answer: tokio::sync::oneshot::Receiver<()>,
    stopped: impl std::future::Future<Output = ()>,
    longest: std::time::Duration,
) -> Result<(), pctwin_quicksend::QuickError> {
    tokio::select! {
        biased;
        () = stopped => Err(pctwin_quicksend::QuickError::NotAnswered),
        a = answer => a.map_err(|_| pctwin_quicksend::QuickError::NotAnswered),
        () = tokio::time::sleep(longest) => Err(pctwin_quicksend::QuickError::NotAnswered),
    }
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
/// session, or else the system's Downloads folder (see [`hold_for_receive`]).
#[tauri::command]
pub fn quicksend_folder<R: Runtime>(
    app: AppHandle<R>,
    qs: State<'_, AppQuickSend>,
) -> Result<FolderView, MoveProblem> {
    hold_for_receive(&qs, app.path().download_dir().ok())
}

/// Holds the folder for the next receive. The folder chosen this session is opened again at its
/// path, and used only while it is still that very folder and not a link: otherwise it is
/// forgotten and nothing is held, so the screens ask the person to choose a folder (files never
/// go somewhere the page did not name). Without a chosen folder, `downloads`, held the same way
/// against what it was the first time this session.
fn hold_for_receive(qs: &QuickSend, downloads: Option<PathBuf>) -> Result<FolderView, MoveProblem> {
    let remembered = lock(&qs.remembered).clone();
    if let Some(known) = remembered {
        return match held_folder(qs, &known.path, Some(known.identity)) {
            Some((view, _)) => Ok(view),
            None => {
                lock(&qs.remembered).take();
                Err(MoveProblem::Failed)
            }
        };
    }
    let downloads = downloads.ok_or(MoveProblem::Failed)?;
    let expect = *lock(&qs.downloads);
    let (view, identity) = held_folder(qs, &downloads, expect).ok_or(MoveProblem::Failed)?;
    lock(&qs.downloads).get_or_insert(identity);
    Ok(view)
}

/// The folder held for this receive, taken so the next receive holds its own: never anywhere
/// else when none is held.
pub(crate) fn take_held(qs: &QuickSend) -> Result<Held, MoveProblem> {
    lock(&qs.folder).take().ok_or(MoveProblem::Failed)
}

/// Change: a folder chosen in the system's own picker, held for the next receive in place of the
/// one shown, and remembered for the rest of the session. `None` when none was chosen.
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
    Ok(Some(remember_chosen(&qs, folder)?))
}

/// Holds a folder the person chose and remembers it, by path and identity, for the session.
fn remember_chosen(qs: &QuickSend, folder: PathBuf) -> Result<FolderView, MoveProblem> {
    let (view, identity) = held_folder(qs, &folder, None).ok_or(MoveProblem::Failed)?;
    *lock(&qs.remembered) = Some(Known {
        path: folder,
        identity,
    });
    Ok(view)
}

/// Whether `path` is itself a plain folder: not a symbolic link, junction or other reparse point
/// (a link could lead anywhere, the system's Startup folder say, while the page names the link).
fn plain_folder(path: &std::path::Path) -> bool {
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return false;
    };
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return false;
        }
    }
    meta.is_dir() && !meta.file_type().is_symlink()
}

/// Opens the plain folder at `path` and says what it is. `None` when it is not a plain folder,
/// cannot be opened, or stopped being a plain folder while it was opened.
fn open_plain(path: &std::path::Path) -> Option<(pctwin_gate::Destination, FileId)> {
    open_plain_around(path, || (), || ())
}

/// [`open_plain`], with what happens just before and just after the open (for tests, which swap
/// a link in and out there). The open follows a link, so what it opened must be the very folder
/// that a second open of the path, which never follows one, finds there: a link swapped in for
/// the open and out again afterwards fails that, whenever it happens.
fn open_plain_around(
    path: &std::path::Path,
    before: impl FnOnce(),
    after: impl FnOnce(),
) -> Option<(pctwin_gate::Destination, FileId)> {
    if !plain_folder(path) {
        return None;
    }
    before();
    let dest = pctwin_gate::Destination::open(path).ok()?;
    after();
    let identity = dest.folder_identity("").ok()??;
    (plain_identity(path)? == identity).then_some((dest, identity))
}

/// What the plain folder at `path` is, from an open that never follows a link: `None` when a
/// link, junction or anything but a plain folder is there.
fn plain_identity(path: &std::path::Path) -> Option<FileId> {
    #[cfg(windows)]
    let folder = {
        use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
        const FILE_READ_ATTRIBUTES: u32 = 0x0080;
        // A folder opens only with this; the second opens a link itself, never what it leads to.
        const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
        const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        let folder = std::fs::OpenOptions::new()
            .access_mode(FILE_READ_ATTRIBUTES)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(path)
            .ok()?;
        let meta = folder.metadata().ok()?;
        if !meta.is_dir() || meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return None;
        }
        folder
    };
    #[cfg(unix)]
    let folder = {
        use rustix::fs::{Mode, OFlags};
        // A link at the name fails the open itself; anything but a folder fails it too.
        std::fs::File::from(
            rustix::fs::open(
                path,
                OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::DIRECTORY | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .ok()?,
        )
    };
    pctwin_gate::file_identity(&folder).ok()
}

/// Opens the folder at `folder` and holds it for the next receive, replacing any held before;
/// what the screens show of it, and what it is. `None` (and nothing held) when it is not a plain
/// folder, cannot be opened, or is not the folder `expect` says it was.
fn held_folder(
    qs: &QuickSend,
    folder: &std::path::Path,
    expect: Option<FileId>,
) -> Option<(FolderView, FileId)> {
    lock(&qs.folder).take();
    let (dest, identity) = open_plain(folder)?;
    if expect.is_some_and(|e| e != identity) {
        return None;
    }
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
        identity,
        dest,
    });
    Some((view, identity))
}

/// Open folder, on the Done page: shows the folder the last receive saved into in the system's
/// file manager (see [`folder_to_open`]).
#[tauri::command]
pub fn quicksend_open_folder<R: Runtime>(
    app: AppHandle<R>,
    qs: State<'_, AppQuickSend>,
) -> Result<(), MoveProblem> {
    use tauri_plugin_opener::OpenerExt;
    let folder = folder_to_open(&qs)?;
    app.opener()
        .open_path(folder.to_string_lossy(), None::<&str>)
        .map_err(|_| MoveProblem::Failed)
}

/// The folder the last receive saved into, only while it is still that very folder and a plain
/// one: never a received file, a link, another folder put at the path, or a path from the screens.
fn folder_to_open(qs: &QuickSend) -> Result<PathBuf, MoveProblem> {
    let known = lock(&qs.received_into)
        .clone()
        .ok_or(MoveProblem::NotMoving)?;
    match open_plain(&known.path) {
        Some((_, identity)) if identity == known.identity => Ok(known.path),
        _ => Err(MoveProblem::Failed),
    }
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

    /// Nothing is received before the person says where: Save goes on; a stop, a Save that can no
    /// longer come, or the sender's wait running out ends it as not answered (29 October 2026).
    #[tokio::test]
    async fn a_receive_waits_for_save() {
        let long = std::time::Duration::from_secs(600);
        let (tx, rx) = tokio::sync::oneshot::channel();
        tx.send(()).unwrap();
        assert!(
            wait_for_save(rx, std::future::pending(), long)
                .await
                .is_ok()
        );
        let (_tx, rx) = tokio::sync::oneshot::channel::<()>();
        assert!(matches!(
            wait_for_save(rx, std::future::ready(()), long).await,
            Err(pctwin_quicksend::QuickError::NotAnswered)
        ));
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        drop(tx);
        assert!(
            wait_for_save(rx, std::future::pending(), long)
                .await
                .is_err()
        );
        let (_tx, rx) = tokio::sync::oneshot::channel::<()>();
        assert!(
            wait_for_save(
                rx,
                std::future::pending(),
                std::time::Duration::from_millis(5)
            )
            .await
            .is_err()
        );
    }

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
        // Its own folder: removing the shared one would pull it from under the tests beside it.
        let dir = dir("pick");
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

    /// A link put where a folder was: a junction on Windows (made with `mklink /J`, which needs
    /// no administrator), a symbolic link elsewhere.
    fn link(at: &std::path::Path, to: &std::path::Path) {
        #[cfg(windows)]
        {
            let made = pctwin_types::process::quiet("cmd")
                .args(["/C", "mklink", "/J"])
                .arg(at)
                .arg(to)
                .output()
                .unwrap();
            assert!(made.status.success(), "mklink /J: {made:?}");
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink(to, at).unwrap();
    }

    fn dir(name: &str) -> PathBuf {
        let d = tempfile_dir().join(name);
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn known(path: &std::path::Path) -> Known {
        Known {
            path: path.to_path_buf(),
            identity: open_plain(path).unwrap().1,
        }
    }

    /// Exit-gate review (24 October 2026) and PR #108 review: the folder is held open from when it
    /// is shown, and taken by the receive, so the next receive holds its folder again at the
    /// remembered path, and only if it is still that very folder.
    #[test]
    fn the_folder_is_held_from_when_it_is_shown_and_held_again_for_each_receive() {
        let qs = QuickSend::default();
        let d = dir("held");
        let chosen = d.join("Inbox");
        std::fs::create_dir_all(&chosen).unwrap();
        let view = remember_chosen(&qs, chosen.clone()).unwrap();
        assert_eq!(view.name, "Inbox");
        let first = take_held(&qs).unwrap();
        assert_eq!(first.path, chosen);
        assert!(lock(&qs.folder).is_none(), "taken by the receive");
        drop(first);
        // Still the same folder: held again.
        hold_for_receive(&qs, None).unwrap();
        take_held(&qs).unwrap();
        // Swapped: the folder moved away and a new one made at its path.
        std::fs::rename(&chosen, d.join("Inbox-moved")).unwrap();
        std::fs::create_dir_all(&chosen).unwrap();
        assert!(
            hold_for_receive(&qs, None).is_err(),
            "another folder at the path"
        );
        assert!(lock(&qs.folder).is_none(), "nothing held");
        assert!(
            lock(&qs.remembered).is_none(),
            "forgotten: the person chooses again"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    /// PR #108 review: a link put at the remembered path is refused even when it leads to the
    /// very folder chosen (the identity matches; the link could be changed to lead anywhere).
    #[test]
    fn a_link_at_the_remembered_path_is_refused_and_forgotten() {
        let qs = QuickSend::default();
        let d = dir("linked");
        let chosen = d.join("Inbox");
        std::fs::create_dir_all(&chosen).unwrap();
        remember_chosen(&qs, chosen.clone()).unwrap();
        take_held(&qs).unwrap();
        let moved = d.join("Inbox-moved");
        std::fs::rename(&chosen, &moved).unwrap();
        link(&chosen, &moved);
        assert!(!plain_folder(&chosen));
        assert!(hold_for_receive(&qs, None).is_err());
        assert!(lock(&qs.remembered).is_none());
        // And a link is never chosen in the first place.
        assert!(remember_chosen(&qs, chosen.clone()).is_err());
        assert!(lock(&qs.folder).is_none());
        let _ = std::fs::remove_dir(&chosen);
        let _ = std::fs::remove_dir_all(&d);
    }

    /// PR #108 review: a remembered folder that is gone is forgotten (so the next receive goes to
    /// Downloads, said on the page), never kept to fail again or swapped silently.
    #[test]
    fn a_remembered_folder_that_is_gone_is_forgotten() {
        let qs = QuickSend::default();
        let d = dir("gone");
        let chosen = d.join("Inbox");
        let downloads = d.join("Downloads");
        std::fs::create_dir_all(&chosen).unwrap();
        std::fs::create_dir_all(&downloads).unwrap();
        remember_chosen(&qs, chosen.clone()).unwrap();
        take_held(&qs).unwrap();
        std::fs::remove_dir_all(&chosen).unwrap();
        assert!(hold_for_receive(&qs, Some(downloads.clone())).is_err());
        assert!(lock(&qs.remembered).is_none(), "forgotten");
        let view = hold_for_receive(&qs, Some(downloads.clone())).unwrap();
        assert_eq!(view.name, "Downloads");
        assert_eq!(take_held(&qs).unwrap().path, downloads);
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Downloads is held against what it was the first time this session: swapped since, refused.
    #[test]
    fn a_swapped_downloads_folder_is_refused() {
        let qs = QuickSend::default();
        let d = dir("downloads");
        let downloads = d.join("Downloads");
        std::fs::create_dir_all(&downloads).unwrap();
        hold_for_receive(&qs, Some(downloads.clone())).unwrap();
        take_held(&qs).unwrap();
        std::fs::rename(&downloads, d.join("Downloads-moved")).unwrap();
        std::fs::create_dir_all(&downloads).unwrap();
        assert!(hold_for_receive(&qs, Some(downloads.clone())).is_err());
        assert!(hold_for_receive(&qs, None).is_err(), "no Downloads at all");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// With no folder held, a receive has nowhere to write: never a folder of its own choosing.
    #[test]
    fn with_no_folder_held_a_receive_has_nowhere_to_write() {
        let qs = QuickSend::default();
        assert!(take_held(&qs).is_err());
        let d = dir("taken");
        remember_chosen(&qs, d.clone()).unwrap();
        assert!(take_held(&qs).is_ok());
        assert!(take_held(&qs).is_err(), "each receive holds its own");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// PR #108 review: Open folder opens the folder the receive saved into, only while it is that
    /// very folder: not a file put at its path, not another folder, not a link to it.
    /// Received files show the arrival time (26 October 2026); Keep original dates gives each
    /// back the date it came with, once. A file that was already there is the person's own and
    /// is never touched.
    #[test]
    fn received_files_show_the_arrival_time_and_can_keep_their_original_dates() {
        let d = dir("dates");
        let old = std::time::UNIX_EPOCH + Duration::from_secs(1_000_000_000);
        let theirs = std::time::UNIX_EPOCH + Duration::from_secs(1_100_000_000);
        let now = std::time::UNIX_EPOCH + Duration::from_secs(1_800_000_000);
        let stamp = |name: &str, time| {
            let f = std::fs::File::options()
                .write(true)
                .open(d.join(name))
                .unwrap();
            f.set_modified(time).unwrap();
        };
        let date = |name: &str| std::fs::metadata(d.join(name)).unwrap().modified().unwrap();
        std::fs::write(d.join("old photo.jpg"), b"jpeg").unwrap();
        stamp("old photo.jpg", old);
        std::fs::write(d.join("mine.txt"), b"mine").unwrap();
        stamp("mine.txt", theirs);
        let id = ItemId::from_hex(&"0".repeat(32)).unwrap();
        let r = Receipt {
            lines: vec![
                pctwin_quicksend::ReceiptLine {
                    id,
                    path: "old photo.jpg".into(),
                    outcome: Outcome::Arrived {
                        landed_as: "old photo.jpg".into(),
                    },
                },
                pctwin_quicksend::ReceiptLine {
                    id,
                    path: "mine.txt".into(),
                    outcome: Outcome::AlreadyThere {
                        landed_as: "mine.txt".into(),
                    },
                },
            ],
        };
        let qs = QuickSend::default();
        let into = known(&d);
        let dest = reopened(&into).unwrap();
        let dated = arrival_dates(&dest, &r, now);
        assert_eq!(dated.len(), 1, "only the file this receive wrote");
        assert_eq!(date("old photo.jpg"), now, "shows when it arrived");
        assert_eq!(
            date("mine.txt"),
            theirs,
            "the person's own file is untouched"
        );
        *lock(&qs.received_into) = Some(into);
        *lock(&qs.dated) = dated;
        assert_eq!(keep_original_dates(&qs).unwrap(), 1);
        assert_eq!(date("old photo.jpg"), old, "its original date is back");
        assert_eq!(date("mine.txt"), theirs);
        assert!(keep_original_dates(&qs).is_err(), "once only");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn open_folder_opens_only_the_folder_received_into() {
        let qs = QuickSend::default();
        assert_eq!(folder_to_open(&qs), Err(MoveProblem::NotMoving));
        let d = dir("open");
        let inbox = d.join("Inbox");
        std::fs::create_dir_all(&inbox).unwrap();
        *lock(&qs.received_into) = Some(known(&inbox));
        assert_eq!(folder_to_open(&qs), Ok(inbox.clone()));
        let moved = d.join("Inbox-moved");
        std::fs::rename(&inbox, &moved).unwrap();
        // A file at the path.
        std::fs::write(&inbox, b"run me").unwrap();
        assert_eq!(folder_to_open(&qs), Err(MoveProblem::Failed));
        std::fs::remove_file(&inbox).unwrap();
        // Another folder at the path.
        std::fs::create_dir_all(&inbox).unwrap();
        assert_eq!(folder_to_open(&qs), Err(MoveProblem::Failed));
        std::fs::remove_dir(&inbox).unwrap();
        // A link at the path, to the very folder.
        link(&inbox, &moved);
        assert_eq!(folder_to_open(&qs), Err(MoveProblem::Failed));
        let _ = std::fs::remove_dir(&inbox);
        let _ = std::fs::remove_dir_all(&d);
    }

    /// A folder that cannot be opened is never held.
    #[test]
    fn a_folder_that_cannot_be_opened_is_not_held() {
        let qs = QuickSend::default();
        let d = dir("never-made");
        assert!(held_folder(&qs, &d.join("nope"), None).is_none());
        assert!(lock(&qs.folder).is_none());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn no_room_is_said_with_the_sizes() {
        assert_eq!(
            problem_of(&pctwin_quicksend::QuickError::NoRoom {
                needed: 238,
                room: 120
            }),
            QsProblem::NoRoom {
                needed: 238,
                room: 120
            }
        );
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

    /// PR #108 re-review: a link swapped in just for the open, and the real folder put back
    /// right after, so every look by name sees a plain folder. The folder opened was the link's
    /// target, so it is refused; with nothing swapped, the folder opens.
    #[test]
    fn a_link_swapped_in_only_for_the_open_is_refused() {
        let d = dir("swap-for-open");
        let real = d.join("Downloads");
        let aside = d.join("Downloads-aside");
        let elsewhere = d.join("Startup");
        std::fs::create_dir_all(&real).unwrap();
        std::fs::create_dir_all(&elsewhere).unwrap();
        let opened = open_plain_around(
            &real,
            || {
                std::fs::rename(&real, &aside).unwrap();
                link(&real, &elsewhere);
            },
            || {
                // Removes the link itself, never what it leads to.
                std::fs::remove_dir(&real)
                    .or_else(|_| std::fs::remove_file(&real))
                    .unwrap();
                std::fs::rename(&aside, &real).unwrap();
            },
        );
        assert!(
            opened.is_none(),
            "the link's target was opened, so it is refused"
        );
        assert!(plain_folder(&real), "the real folder is back");
        assert!(open_plain(&real).is_some(), "with nothing swapped it opens");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_link_or_a_file_has_no_plain_identity() {
        let d = dir("plain-identity");
        let real = d.join("real");
        std::fs::create_dir_all(&real).unwrap();
        let file = d.join("file.txt");
        std::fs::write(&file, b"x").unwrap();
        let linked = d.join("linked");
        link(&linked, &real);
        assert!(plain_identity(&real).is_some());
        assert_eq!(plain_identity(&real), open_plain(&real).map(|(_, id)| id));
        assert!(
            plain_identity(&linked).is_none(),
            "a link is not a plain folder"
        );
        assert!(plain_identity(&file).is_none(), "a file is not a folder");
        assert!(plain_identity(&d.join("gone")).is_none());
        let _ = std::fs::remove_dir_all(&d);
    }

    fn tempfile_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pctwin-qs-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
