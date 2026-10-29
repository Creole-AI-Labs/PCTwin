//! Remembered connections, their permissions and waiting sends, for the screens (Security Design
//! Part T, Quick Send Stage 1). The engine is `pctwin-known` (the book, keys, folders) and
//! `pctwin-quicksend` (notes, waiting sends); this module runs them:
//!
//! - After an exchange over the six-digit pairing, the two laptops have swapped public keys
//!   ([`swap_keys`]); the screens may offer "Remember this connection" ([`known_remember`]).
//! - While any connection is remembered, a quiet service listens for remembered laptops (Noise
//!   KK, local network only) and announces one changing token per connection, never a name.
//! - Files from a remembered laptop are asked about every time (permission 2), unless permission
//!   3 lets them into its folder: then they land in a PCTwin-named folder inside it, within its
//!   caps, and the person is told on the first one after a quiet spell.
//! - Waiting sends are delivered by the same service while this laptop is on, each only to its
//!   recipient's key.
//!
//! The screens never name a path: folders are chosen in the system's picker here.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use pctwin_known::{Book, BookError, FolderProblem, GrantProblem, KnownFolder, NextFolder};
use pctwin_link::known::{KnownListener, Remembered as RememberedKeys, connect_known};
use pctwin_link::{Link, LinkConfig};
use pctwin_move::Live;
use pctwin_quicksend::waiting::{Delivery, WaitingError, WaitingInfo, WaitingStore};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

use crate::quicksend::{
    AppQuickSend, FolderView, LineView, OfferedView, PROBLEM_EVENT, RECEIPT_EVENT, lock, offered,
    problem_of, receipt_view, shown_safely,
};

/// Files from a remembered laptop wait for the person: Save or Decline.
pub const INCOMING_EVENT: &str = "known-incoming";
/// Files from a remembered laptop landed under permission 3.
pub const DELIVERED_EVENT: &str = "known-delivered";
/// The remembered connections changed (one expired, or was used): the whole list.
pub const CHANGED_EVENT: &str = "known-changed";
/// The waiting sends changed (one delivered, declined, or past its deadline): the whole list.
pub const WAITING_EVENT: &str = "waiting-changed";

/// How often the service looks for recipients of waiting sends, and tidies.
const ROUND: Duration = Duration::from_secs(60);
/// How long one look for a remembered laptop nearby lasts.
const LOOK: Duration = Duration::from_secs(3);

pctwin_types::listed_enum! {
    /// Why a Quick Send command was refused, as a translation key.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(tag = "kind", rename_all = "kebab-case")]
    pub enum QsRefusal {
        /// The laptops are not paired (any more): pair again.
        NotPaired,
        /// Nothing is waiting for this (no offer, no files arriving).
        NotWaiting,
        Failed,
        NoteEmpty,
        NoteTooLong,
        NoteTooManyLines,
        /// The note holds a character that cannot be shown safely.
        NoteHidden,
        /// The last note was not a link.
        NoLink,
        /// No such connection, or waiting send.
        NotFound,
        /// This laptop was restored or copied since the connection was remembered: pair again.
        Restored,
        /// The name is empty or too long.
        BadName,
        /// The system's key store could not be reached.
        StoreUnavailable,
        TooManyConnections,
        FolderRunsAtSignIn,
        FolderShellStartUp,
        FolderHoldsKeys,
        FolderSystem,
        FolderNotPlain,
        FolderNotTheSame,
        /// The remembered laptop is not nearby now (it may be off, or on another network).
        NotNearby,
        /// The waiting sends would pass their total limit.
        OverCap,
        /// Not enough free space for the copies.
        NoRoom,
        /// The deadline must be 1 to 30 days.
        BadDays,
        /// Recipients: none, too many, or one twice.
        BadRecipients,
        /// Too many files for one send.
        TooManyFiles,
        /// A file changed while it was being copied: try again.
        ChangedWhileCopying,
    }
}

pub(crate) fn note_refusal(e: pctwin_quicksend::NoteError) -> QsRefusal {
    match e {
        pctwin_quicksend::NoteError::Empty => QsRefusal::NoteEmpty,
        pctwin_quicksend::NoteError::TooLong => QsRefusal::NoteTooLong,
        pctwin_quicksend::NoteError::TooManyLines => QsRefusal::NoteTooManyLines,
        pctwin_quicksend::NoteError::Hidden => QsRefusal::NoteHidden,
    }
}

fn book_refusal(e: BookError) -> QsRefusal {
    match e {
        BookError::NotFound => QsRefusal::NotFound,
        BookError::Store(_) => QsRefusal::StoreUnavailable,
        BookError::File => QsRefusal::Failed,
        BookError::Restored => QsRefusal::Restored,
        BookError::BadLabel => QsRefusal::BadName,
        BookError::Full => QsRefusal::TooManyConnections,
    }
}

fn folder_refusal(e: FolderProblem) -> QsRefusal {
    match e {
        FolderProblem::RunsAtSignIn => QsRefusal::FolderRunsAtSignIn,
        FolderProblem::ShellStartUp => QsRefusal::FolderShellStartUp,
        FolderProblem::HoldsKeys => QsRefusal::FolderHoldsKeys,
        FolderProblem::System => QsRefusal::FolderSystem,
        FolderProblem::NotPlain => QsRefusal::FolderNotPlain,
        FolderProblem::NotTheSame => QsRefusal::FolderNotTheSame,
    }
}

fn waiting_refusal(e: &WaitingError) -> QsRefusal {
    match e {
        WaitingError::Recipients => QsRefusal::BadRecipients,
        WaitingError::Days => QsRefusal::BadDays,
        WaitingError::OverCap { .. } => QsRefusal::OverCap,
        WaitingError::NoRoom { .. } => QsRefusal::NoRoom,
        WaitingError::ChangedWhileCopying(_) => QsRefusal::ChangedWhileCopying,
        WaitingError::NotFound | WaitingError::PastDeadline => QsRefusal::NotFound,
        WaitingError::Quick(pctwin_quicksend::QuickError::TooMany { .. }) => {
            QsRefusal::TooManyFiles
        }
        _ => QsRefusal::Failed,
    }
}

/// Seconds since 1970, by this laptop's clock.
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// A file from a remembered laptop waiting for the person's answer.
struct Asking {
    connection: String,
    answer: tokio::sync::oneshot::Sender<Answer>,
}

enum Answer {
    Save { next_time: bool },
    Decline,
}

#[derive(Default)]
pub struct RememberedState {
    book: Mutex<Option<Book>>,
    /// The keys swapped in the last six-digit exchange, until the person remembers it or the
    /// next exchange starts.
    pending: Mutex<Option<pctwin_known::Pending>>,
    asking: Mutex<Option<Asking>>,
    /// The sessions running with each connection, so Forget can end them.
    sessions: Mutex<BTreeMap<String, Vec<(u64, Live)>>>,
    next_session: std::sync::atomic::AtomicU64,
    /// The service is running.
    serving: std::sync::atomic::AtomicBool,
}

pub type AppRemembered = Arc<RememberedState>;

pub fn new_state() -> AppRemembered {
    Arc::new(RememberedState::default())
}

fn dir<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    app.path().app_data_dir().ok().map(|d| d.join("quicksend"))
}

fn waiting_store<R: Runtime>(app: &AppHandle<R>) -> Result<WaitingStore, QsRefusal> {
    Ok(WaitingStore::new(
        &dir(app).ok_or(QsRefusal::Failed)?.join("waiting"),
    ))
}

/// Swaps this connection's public keys inside the sealed six-digit link (both laptops send,
/// then read): what "Remember this connection" keeps. A message not in its form gives nothing
/// to remember, and the exchange goes on.
pub(crate) async fn swap_keys(
    link: &mut Link,
) -> Result<Option<pctwin_known::Pending>, pctwin_quicksend::QuickError> {
    let dropped = || {
        pctwin_quicksend::QuickError::Transfer(pctwin_transfer::TransferError::ConnectionDropped)
    };
    let swap = pctwin_known::Swap::new().ok();
    let mine = swap
        .as_ref()
        .map_or_else(|| vec![0], pctwin_known::Swap::message);
    link.send(&mine).await.map_err(|_| dropped())?;
    let theirs = link.recv().await.map_err(|_| dropped())?;
    Ok(swap.and_then(|s| s.finish(&theirs).ok()))
}

/// "Remember this connection" offered after a six-digit exchange: whether the keys would be kept
/// in the weaker file fallback (Linux), said plainly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RememberView {
    pub fallback_store: bool,
}

impl RememberedState {
    fn with_book<R: Runtime, T>(
        &self,
        app: &AppHandle<R>,
        f: impl FnOnce(&mut Book) -> Result<T, BookError>,
    ) -> Result<T, QsRefusal> {
        let mut book = lock(&self.book);
        if book.is_none() {
            let path = dir(app).ok_or(QsRefusal::Failed)?.join("connections.json");
            let opened = Book::open(
                &path,
                Arc::new(pctwin_known::SystemStore),
                pctwin_known::machine::this_machine(),
            )
            .map_err(book_refusal)?;
            *book = Some(opened);
        }
        f(book.as_mut().ok_or(QsRefusal::Failed)?).map_err(book_refusal)
    }

    /// After a six-digit exchange ended well: keeps the swapped keys for "Remember this
    /// connection" and tells the screens.
    pub(crate) fn offer_remember<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        pending: Option<pctwin_known::Pending>,
    ) {
        let Some(pending) = pending else { return };
        *lock(&self.pending) = Some(pending);
        let _ = app.emit(
            crate::quicksend::REMEMBER_EVENT,
            RememberView {
                fallback_store: pctwin_known::SystemStore::is_fallback(),
            },
        );
    }

    fn list<R: Runtime>(&self, app: &AppHandle<R>) -> Result<Vec<ConnectionView>, QsRefusal> {
        let t = now();
        self.with_book(app, |b| Ok(b.list(t)))
            .map(|l| l.into_iter().map(view).collect())
    }

    fn started(&self, connection: &str, live: &Live) -> u64 {
        let n = self
            .next_session
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        lock(&self.sessions)
            .entry(connection.to_string())
            .or_default()
            .push((n, live.clone()));
        n
    }

    fn ended(&self, connection: &str, session: u64) {
        if let Some(v) = lock(&self.sessions).get_mut(connection) {
            v.retain(|(n, _)| *n != session);
        }
    }
}

/// A folder as the connection list shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GrantView {
    pub name: String,
    pub path: String,
    pub ends_at: u64,
    pub ended: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NextFolderView {
    pub name: String,
    pub path: String,
}

/// A remembered connection as the screens list it: "same device as before", never a person.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionView {
    pub id: String,
    /// The name the person gave it, shown as typed.
    pub label: String,
    pub made_at: u64,
    pub last_used: u64,
    pub expires_at: u64,
    /// False on a restored backup or a copied drive: pair again.
    pub same_device: bool,
    /// Permission 3, when given.
    pub folder: Option<GrantView>,
    /// "Folder for next time", when chosen.
    pub next_folder: Option<NextFolderView>,
}

fn folder_name(p: &std::path::Path) -> (String, String) {
    let name = p.file_name().map_or_else(
        || p.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    (shown_safely(&name), shown_safely(&p.display().to_string()))
}

fn view(c: pctwin_known::ConnectionInfo) -> ConnectionView {
    ConnectionView {
        id: c.id,
        label: shown_safely(&c.label),
        made_at: c.made_at,
        last_used: c.last_used,
        expires_at: c.expires_at,
        same_device: c.same_device,
        folder: c.grant.map(|g| {
            let (name, path) = folder_name(&g.folder);
            GrantView {
                name,
                path,
                ends_at: g.ends_at,
                ended: g.ended,
            }
        }),
        next_folder: c.next_folder.map(|p| {
            let (name, path) = folder_name(&p);
            NextFolderView { name, path }
        }),
    }
}

/// Files from a remembered laptop, waiting for Save or Decline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnownIncomingView {
    pub connection: String,
    pub label: String,
    pub files: Vec<OfferedView>,
    pub bytes: u64,
    /// The folder held for it now ("folder for next time", when one is chosen and still that
    /// folder); `None`: the screens hold one (Downloads, or Choose another folder).
    pub folder: Option<FolderView>,
}

/// Files that landed under permission 3.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnownDeliveredView {
    pub connection: String,
    pub label: String,
    /// The allowed folder's name (they are in its "PCTwin Quick Send" folder).
    pub folder: String,
    pub lines: Vec<LineView>,
    /// The first delivery after a quiet spell: say so plainly.
    pub tell: bool,
}

/// One recipient of a waiting send, as the screens show it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipientView {
    pub connection: String,
    /// The connection's name now (`None`: forgotten since).
    pub label: Option<String>,
    pub delivery: Delivery,
}

/// A waiting send as the screens list it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WaitingView {
    pub id: String,
    pub made_at: u64,
    pub deadline: u64,
    pub files: u64,
    pub bytes: u64,
    pub recipients: Vec<RecipientView>,
    /// Files changed since the approval: "a newer version exists, review again".
    pub newer: Vec<String>,
}

fn waiting_view(w: WaitingInfo, labels: &BTreeMap<String, String>) -> WaitingView {
    WaitingView {
        id: w.id,
        made_at: w.made_at,
        deadline: w.deadline,
        files: w.files,
        bytes: w.bytes,
        recipients: w
            .recipients
            .into_iter()
            .map(|r| RecipientView {
                label: labels.get(&r.connection).map(|l| shown_safely(l)),
                connection: r.connection,
                delivery: r.delivery,
            })
            .collect(),
        newer: w.newer.iter().map(|p| shown_safely(p)).collect(),
    }
}

fn waiting_views<R: Runtime>(
    app: &AppHandle<R>,
    st: &RememberedState,
) -> Result<Vec<WaitingView>, QsRefusal> {
    let labels: BTreeMap<String, String> = st
        .list(app)
        .unwrap_or_default()
        .into_iter()
        .map(|c| (c.id, c.label))
        .collect();
    Ok(waiting_store(app)?
        .list()
        .into_iter()
        .map(|w| waiting_view(w, &labels))
        .collect())
}

/* ---------- Commands: connections and permissions ---------- */

#[tauri::command]
pub fn known_list<R: Runtime>(
    app: AppHandle<R>,
    st: State<'_, AppRemembered>,
) -> Result<Vec<ConnectionView>, QsRefusal> {
    st.list(&app)
}

/// Remember this connection (permission 1), named `label` by the person, from the keys swapped
/// in the last six-digit exchange. The service starts if it was not running.
#[tauri::command]
pub fn known_remember<R: Runtime>(
    app: AppHandle<R>,
    st: State<'_, AppRemembered>,
    label: String,
) -> Result<Vec<ConnectionView>, QsRefusal> {
    let pending = lock(&st.pending).take().ok_or(QsRefusal::NotWaiting)?;
    let t = now();
    st.with_book(&app, |b| b.remember(pending, &label, t))?;
    start_service(&app);
    st.list(&app)
}

/// Forget this connection: its key and every permission are deleted, its waiting deliveries end,
/// and any transfer with it now stops (PCTwin's own part files removed). Files already received
/// stay where they are.
#[tauri::command]
pub fn known_forget<R: Runtime>(
    app: AppHandle<R>,
    st: State<'_, AppRemembered>,
    id: String,
) -> Result<Vec<ConnectionView>, QsRefusal> {
    st.with_book(&app, |b| b.forget(&id))?;
    if let Some(lives) = lock(&st.sessions).remove(&id) {
        for (_, l) in lives {
            l.control.stop();
        }
    }
    {
        let mut asking = lock(&st.asking);
        if asking.as_ref().is_some_and(|a| a.connection == id)
            && let Some(a) = asking.take()
        {
            let _ = a.answer.send(Answer::Decline);
        }
    }
    if let Ok(store) = waiting_store(&app) {
        store.forget_recipient(&id);
    }
    let _ = app.emit(WAITING_EVENT, waiting_views(&app, &st).unwrap_or_default());
    st.list(&app)
}

#[tauri::command]
pub fn known_rename<R: Runtime>(
    app: AppHandle<R>,
    st: State<'_, AppRemembered>,
    id: String,
    label: String,
) -> Result<Vec<ConnectionView>, QsRefusal> {
    st.with_book(&app, |b| b.rename(&id, &label))?;
    st.list(&app)
}

/// Permission 3: allow future sends from this connection into a folder chosen in the system's
/// picker (never a place that runs things or holds keys). `None` when none was chosen.
#[tauri::command]
pub async fn known_allow_folder<R: Runtime>(
    app: AppHandle<R>,
    st: State<'_, AppRemembered>,
    id: String,
) -> Result<Option<Vec<ConnectionView>>, QsRefusal> {
    let chosen = tauri::async_runtime::spawn_blocking(|| rfd::FileDialog::new().pick_folder())
        .await
        .map_err(|_| QsRefusal::Failed)?;
    let Some(path) = chosen else { return Ok(None) };
    let (folder, _) = KnownFolder::choose(&path).map_err(folder_refusal)?;
    let t = now();
    st.with_book(&app, |b| b.allow_folder(&id, folder, t))?;
    st.list(&app).map(Some)
}

/// Withdraws permission 3 for this connection (the connection stays).
#[tauri::command]
pub fn known_revoke_folder<R: Runtime>(
    app: AppHandle<R>,
    st: State<'_, AppRemembered>,
    id: String,
) -> Result<Vec<ConnectionView>, QsRefusal> {
    st.with_book(&app, |b| b.revoke_folder(&id))?;
    st.list(&app)
}

/// Renews permission 3 for another 30 days from now.
#[tauri::command]
pub fn known_renew_folder<R: Runtime>(
    app: AppHandle<R>,
    st: State<'_, AppRemembered>,
    id: String,
) -> Result<Vec<ConnectionView>, QsRefusal> {
    let t = now();
    st.with_book(&app, |b| b.renew_folder(&id, t))?;
    st.list(&app)
}

/// "Use this folder for this connection next time": the folder the last receive saved into,
/// only while it is still that very folder and not a refused place.
#[tauri::command]
pub fn known_next_folder<R: Runtime>(
    app: AppHandle<R>,
    st: State<'_, AppRemembered>,
    qs: State<'_, AppQuickSend>,
    id: String,
) -> Result<Vec<ConnectionView>, QsRefusal> {
    let into = lock(&qs.received_into)
        .clone()
        .ok_or(QsRefusal::NotWaiting)?;
    let (folder, _) = KnownFolder::choose(&into.path).map_err(folder_refusal)?;
    if folder.identity != into.identity {
        return Err(QsRefusal::FolderNotTheSame);
    }
    st.with_book(&app, |b| b.set_next_folder(&id, NextFolder(folder)))?;
    st.list(&app)
}

#[tauri::command]
pub fn known_clear_next_folder<R: Runtime>(
    app: AppHandle<R>,
    st: State<'_, AppRemembered>,
    id: String,
) -> Result<Vec<ConnectionView>, QsRefusal> {
    st.with_book(&app, |b| b.clear_next_folder(&id))?;
    st.list(&app)
}

/// Save, for files from a remembered laptop: they go into the folder held now (the folder for
/// next time, Downloads, or one chosen), and with `nextTime` that folder becomes this
/// connection's folder for next time.
#[tauri::command]
pub fn known_save_incoming(
    st: State<'_, AppRemembered>,
    qs: State<'_, AppQuickSend>,
    next_time: bool,
) -> Result<(), QsRefusal> {
    if lock(&qs.folder).is_none() {
        return Err(QsRefusal::Failed);
    }
    let a = lock(&st.asking).take().ok_or(QsRefusal::NotWaiting)?;
    a.answer
        .send(Answer::Save { next_time })
        .map_err(|_| QsRefusal::NotWaiting)
}

/// Decline, for files from a remembered laptop: nothing is written; the sender is told.
#[tauri::command]
pub fn known_decline_incoming(st: State<'_, AppRemembered>) -> Result<(), QsRefusal> {
    let a = lock(&st.asking).take().ok_or(QsRefusal::NotWaiting)?;
    a.answer
        .send(Answer::Decline)
        .map_err(|_| QsRefusal::NotWaiting)
}

/// Sends the offer made on the pick page to a remembered laptop nearby, now (live, over the
/// known-keys session); progress and the receipt come as for a six-digit send.
#[tauri::command]
pub async fn known_send<R: Runtime>(
    app: AppHandle<R>,
    st: State<'_, AppRemembered>,
    qs: State<'_, AppQuickSend>,
    id: String,
) -> Result<(), QsRefusal> {
    let offer = lock(&qs.offer).take().ok_or(QsRefusal::NotWaiting)?;
    let peer = st.with_book(&app, |b| b.peer_public(&id))?;
    let mut link = match connect_to(&app, &st, &id, &peer).await {
        Ok(link) => link,
        Err(e) => {
            *lock(&qs.offer) = Some(offer);
            return Err(e);
        }
    };
    let state = st.inner().clone();
    let live = Live::default();
    *lock(&qs.live) = Some(live.clone());
    let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
    crate::quicksend::tick(app.clone(), live.clone(), done.clone());
    let session = state.started(&id, &live);
    let runtime = tokio::runtime::Handle::current();
    let spawned = std::thread::Builder::new()
        .name("pctwin-knownsend".into())
        .spawn(move || {
            let sent = runtime.block_on(pctwin_quicksend::send_live(&mut link, offer, &live));
            done.store(true, std::sync::atomic::Ordering::Release);
            state.ended(&id, session);
            match sent {
                Ok(r) => {
                    let view = receipt_view("sender", &r, live.control.stopped());
                    let _ = app.emit(RECEIPT_EVENT, view);
                }
                Err(e) => {
                    let _ = app.emit(PROBLEM_EVENT, problem_of(&e));
                }
            }
        });
    spawned.map(|_| ()).map_err(|_| QsRefusal::Failed)
}

/* ---------- Commands: waiting sends ---------- */

/// Approve this delivery only: the offer made on the pick page, copied now, for each of the
/// remembered connections `ids` (their keys as they are now), until `days` days from now.
#[tauri::command]
pub async fn waiting_prepare<R: Runtime>(
    app: AppHandle<R>,
    st: State<'_, AppRemembered>,
    qs: State<'_, AppQuickSend>,
    ids: Vec<String>,
    days: u64,
) -> Result<Vec<WaitingView>, QsRefusal> {
    let chosen = lock(&qs.chosen).clone();
    if chosen.is_empty() {
        return Err(QsRefusal::NotWaiting);
    }
    let mut recipients = Vec::new();
    for id in &ids {
        let key = st.with_book(&app, |b| {
            b.keys_for(id)?;
            b.peer_public(id)
        })?;
        recipients.push((id.clone(), key));
    }
    let store = waiting_store(&app)?;
    let root = dir(&app).ok_or(QsRefusal::Failed)?;
    let free = std::fs::create_dir_all(&root)
        .ok()
        .and_then(|()| pctwin_gate::Destination::open(&root).ok())
        .and_then(|d| d.available_bytes());
    tauri::async_runtime::spawn_blocking(move || {
        store.prepare(&chosen, &recipients, days, now(), free)
    })
    .await
    .map_err(|_| QsRefusal::Failed)?
    .map_err(|e| waiting_refusal(&e))?;
    start_service(&app);
    waiting_views(&app, &st)
}

#[tauri::command]
pub fn waiting_list<R: Runtime>(
    app: AppHandle<R>,
    st: State<'_, AppRemembered>,
) -> Result<Vec<WaitingView>, QsRefusal> {
    waiting_views(&app, &st)
}

/// Renews a waiting send's deadline to `days` days from now (only when the person asks).
#[tauri::command]
pub fn waiting_renew<R: Runtime>(
    app: AppHandle<R>,
    st: State<'_, AppRemembered>,
    id: String,
    days: u64,
) -> Result<Vec<WaitingView>, QsRefusal> {
    waiting_store(&app)?
        .renew(&id, days, now())
        .map_err(|e| waiting_refusal(&e))?;
    waiting_views(&app, &st)
}

/// Cancels a waiting send: its protected copy is deleted.
#[tauri::command]
pub fn waiting_cancel<R: Runtime>(
    app: AppHandle<R>,
    st: State<'_, AppRemembered>,
    id: String,
) -> Result<Vec<WaitingView>, QsRefusal> {
    waiting_store(&app)?
        .cancel(&id)
        .map_err(|e| waiting_refusal(&e))?;
    waiting_views(&app, &st)
}

/* ---------- The service ---------- */

/// Finds the remembered laptop of connection `id` nearby by its changing token, and opens a
/// known-keys session with it, to the key `peer` only.
async fn connect_to<R: Runtime>(
    app: &AppHandle<R>,
    st: &RememberedState,
    id: &str,
    peer: &[u8; 32],
) -> Result<Link, QsRefusal> {
    let t = now();
    let tokens = st.with_book(app, |b| b.peer_tokens(id, t))?;
    let found = tauri::async_runtime::spawn_blocking(|| {
        pctwin_discovery::Discovery::new()
            .and_then(|d| d.browse_known(LOOK))
            .unwrap_or_default()
    })
    .await
    .map_err(|_| QsRefusal::Failed)?;
    for f in found.into_iter().filter(|f| tokens.contains(&f.token)) {
        for addr in f.addrs {
            let keys = st.with_book(app, |b| b.keys_for(id))?;
            if let Ok(link) =
                connect_known(addr, &f.token, &keys, peer, LinkConfig::default()).await
            {
                let _ = st.with_book(app, |b| b.touch(id, now()));
                return Ok(link);
            }
        }
    }
    Err(QsRefusal::NotNearby)
}

/// At start: the service runs only when something is remembered or waiting.
pub fn start_if_needed<R: Runtime>(app: &AppHandle<R>) {
    let Some(d) = dir(app) else { return };
    let remembered = d.join("connections.json").exists();
    let waiting = std::fs::read_dir(d.join("waiting")).is_ok_and(|mut e| e.next().is_some());
    if remembered || waiting {
        start_service(app);
    }
}

/// Starts the service once: it runs while the app does, doing nothing while nothing is
/// remembered and nothing waits.
pub fn start_service<R: Runtime>(app: &AppHandle<R>) {
    let st = app.state::<AppRemembered>().inner().clone();
    if st.serving.swap(true, std::sync::atomic::Ordering::AcqRel) {
        return;
    }
    let app = app.clone();
    // The transfer engine's sessions run on one thread (they are not sent between threads), so
    // the service has a thread and a runtime of its own.
    let started = std::thread::Builder::new()
        .name("pctwin-known".into())
        .spawn(move || {
            let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            else {
                st.serving
                    .store(false, std::sync::atomic::Ordering::Release);
                return;
            };
            let local = tokio::task::LocalSet::new();
            local.block_on(&rt, serve(app, st));
        });
    // A thread that could not start leaves the service marked running: the app is then too
    // short of resources to serve anyway, and the next start of the app tries again.
    let _ = started;
}

/// The service: answers remembered laptops, announces one token per connection (each its own
/// announcement, replaced when the tokens change), and runs a round every [`ROUND`].
async fn serve<R: Runtime>(app: AppHandle<R>, st: AppRemembered) {
    let any = std::net::SocketAddr::from(([0, 0, 0, 0], 0));
    let Ok(listener) = KnownListener::bind(&[any], LinkConfig::default()).await else {
        st.serving
            .store(false, std::sync::atomic::Ordering::Release);
        return;
    };
    let listener = std::rc::Rc::new(listener);
    let accept = {
        let (app, st, listener) = (app.clone(), st.clone(), listener.clone());
        async move {
            while let Ok(arrival) = listener.accept().await {
                let (app, st) = (app.clone(), st.clone());
                tokio::task::spawn_local(async move {
                    arrived(&app, &st, arrival).await;
                });
            }
        }
    };
    let rounds = async move {
        let discovery = pctwin_discovery::Discovery::new().ok();
        let mut announced: Vec<([u8; 8], pctwin_discovery::Announcement)> = Vec::new();
        loop {
            round(&app, &st).await;
            let tokens: Vec<[u8; 8]> = st
                .with_book(&app, |b| Ok(b.announced(now())))
                .unwrap_or_default()
                .into_iter()
                .map(|(_, t)| t)
                .collect();
            let same = tokens.len() == announced.len()
                && tokens.iter().all(|t| announced.iter().any(|(a, _)| a == t));
            if !same {
                // Goodbye for the old tokens, then the new ones.
                announced.clear();
                if let (Some(d), Ok(port)) = (&discovery, listener.port()) {
                    for t in tokens {
                        if let Ok(a) = d.announce_known(&t, port, &[]) {
                            announced.push((t, a));
                        }
                    }
                }
            }
            tokio::time::sleep(ROUND).await;
        }
    };
    tokio::join!(accept, rounds);
}

/// One round: tidies (expired connections, waiting sends past their deadline) and delivers each
/// waiting send to each recipient still waiting that is nearby.
async fn round<R: Runtime>(app: &AppHandle<R>, st: &RememberedState) {
    let t = now();
    let expired = st.with_book(app, |b| b.expire(t)).unwrap_or_default();
    let Ok(store) = waiting_store(app) else {
        return;
    };
    for id in &expired {
        store.forget_recipient(id);
    }
    let swept = store.sweep(t);
    if !expired.is_empty() {
        let _ = app.emit(CHANGED_EVENT, st.list(app).unwrap_or_default());
    }
    let mut changed = !swept.is_empty() || !expired.is_empty();
    for w in store.list() {
        for r in w
            .recipients
            .iter()
            .filter(|r| r.delivery == Delivery::Waiting)
        {
            let Ok((_, key)) = store.offer_for(&w.id, &r.connection, t) else {
                continue;
            };
            // Bound to the key approved: a connection paired again since has another.
            if st.with_book(app, |b| b.peer_public(&r.connection)).ok() != Some(key) {
                continue;
            }
            let Ok(mut link) = connect_to(app, st, &r.connection, &key).await else {
                continue;
            };
            let live = Live::default();
            let session = st.started(&r.connection, &live);
            let _ = store
                .deliver(&w.id, &r.connection, &mut link, &live, now())
                .await;
            st.ended(&r.connection, session);
            changed = true;
        }
    }
    if changed {
        let _ = app.emit(WAITING_EVENT, waiting_views(app, st).unwrap_or_default());
    }
}

/// A remembered laptop connected: the handshake with the keys its token names, then what it
/// sends.
async fn arrived<R: Runtime>(
    app: &AppHandle<R>,
    st: &AppRemembered,
    arrival: pctwin_link::known::KnownArrival,
) {
    let t = now();
    let shook = arrival
        .handshake(|hint| {
            st.with_book(app, |b| {
                let Some(id) = b.lookup(hint, t) else {
                    return Ok(None);
                };
                Ok(Some(RememberedKeys {
                    local: b.keys_for(&id)?,
                    remote: b.peer_public(&id)?,
                    connection: id,
                }))
            })
            .ok()
            .flatten()
        })
        .await;
    let Ok((id, mut link)) = shook else { return };
    let _ = st.with_book(app, |b| b.touch(&id, now()));
    let label = st.with_book(app, |b| b.label(&id)).unwrap_or_default();
    let qs = app.state::<AppQuickSend>().inner().clone();
    let got = pctwin_quicksend::receive(&mut link).await;
    match got {
        Ok(pctwin_quicksend::Arrival::Note(note)) => {
            crate::quicksend::show_note(app, &qs, note, Some(shown_safely(&label)));
        }
        Ok(pctwin_quicksend::Arrival::Files(incoming)) => {
            let live = Live::default();
            let session = st.started(&id, &live);
            let ended = receive_files(app, st, &qs, &id, &label, incoming, &mut link, &live).await;
            st.ended(&id, session);
            if let Err(e) = ended {
                let _ = app.emit(PROBLEM_EVENT, problem_of(&e));
            }
        }
        Err(_) => {}
    }
}

fn journal<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<(pctwin_journal::Journal, String), pctwin_quicksend::QuickError> {
    let bad = |w: &str| pctwin_quicksend::QuickError::Destination(w.into());
    let me = pctwin_scan::list_people()
        .into_iter()
        .find(|p| p.is_me)
        .map(|p| p.account_id)
        .ok_or_else(|| bad("no account"))?;
    let at = crate::moving::journal_path(app).ok_or_else(|| bad("no journal"))?;
    if let Some(dir) = at.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let j = pctwin_journal::Journal::open(&at).map_err(|e| bad(&e.to_string()))?;
    Ok((j, me))
}

/// The PCTwin-named folder inside permission 3's folder, rechecked: the folder is still that
/// very folder and not a refused place, and the PCTwin folder in it is a plain folder.
fn delivery_folder(folder: &KnownFolder) -> Result<pctwin_gate::Destination, FolderProblem> {
    folder.reopen()?;
    let inside = folder.path.join(pctwin_known::book::DELIVERY_FOLDER);
    match std::fs::create_dir(&inside) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(_) => return Err(FolderProblem::NotPlain),
    }
    let (dest, _) = pctwin_known::folders::open_plain(&inside).ok_or(FolderProblem::NotPlain)?;
    // The allowed folder again, after the PCTwin folder was opened.
    folder.reopen()?;
    Ok(dest)
}

/// Removes PCTwin's own unfinished files a stopped receive left in the folders of `lines`
/// (only `.pctwin-` temporary files: anything else is refused by the gate).
fn remove_part_files(dest: &pctwin_gate::Destination, base: &std::path::Path, paths: &[String]) {
    let mut folders: Vec<String> = paths
        .iter()
        .map(|p| {
            p.rsplit_once('/')
                .map_or(String::new(), |(d, _)| d.to_string())
        })
        .collect();
    folders.sort();
    folders.dedup();
    for f in folders {
        let Ok(list) = std::fs::read_dir(base.join(&f)) else {
            continue;
        };
        for name in list
            .filter_map(Result::ok)
            .filter_map(|e| e.file_name().into_string().ok())
        {
            let stored = if f.is_empty() {
                name
            } else {
                format!("{f}/{name}")
            };
            // Through the folder held open, and only a `.pctwin-` temporary file.
            let _ = dest.remove_temp(&stored);
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn receive_files<R: Runtime>(
    app: &AppHandle<R>,
    st: &AppRemembered,
    qs: &crate::quicksend::QuickSend,
    id: &str,
    label: &str,
    incoming: pctwin_quicksend::Incoming,
    link: &mut Link,
    live: &Live,
) -> Result<(), pctwin_quicksend::QuickError> {
    let files = incoming.files();
    let bytes = incoming.total_bytes();
    let paths: Vec<String> = files.iter().map(|f| f.path.clone()).collect();
    // Permission 3: into its folder, within its caps; rechecked now.
    match st.with_book(app, |b| Ok(b.admit(id, bytes, now()))) {
        Ok(Ok(admitted)) => {
            if let Ok(dest) = delivery_folder(&admitted.folder) {
                let (journal, me) = journal(app)?;
                let reopen = delivery_folder(&admitted.folder).ok();
                let r = incoming
                    .accept_into(link, dest, None, &journal, &me, live)
                    .await?;
                if live.control.stopped()
                    && let Some(d) = reopen
                {
                    let base = admitted
                        .folder
                        .path
                        .join(pctwin_known::book::DELIVERY_FOLDER);
                    remove_part_files(&d, &base, &paths);
                }
                let (folder, _) = folder_name(&admitted.folder.path);
                let _ = app.emit(
                    DELIVERED_EVENT,
                    KnownDeliveredView {
                        connection: id.to_string(),
                        label: shown_safely(label),
                        folder,
                        lines: receipt_view("receiver", &r, live.control.stopped()).lines,
                        tell: admitted.tell,
                    },
                );
                return Ok(());
            }
            // The folder is not usable now: ask the person instead.
        }
        Ok(Err(GrantProblem::None | GrantProblem::Expired | GrantProblem::OverCap)) | Err(_) => {}
    }
    // Permission 2: asked every time.
    let (answer_tx, answer_rx) = tokio::sync::oneshot::channel();
    // One question at a time: another remembered laptop asking meanwhile is declined (its
    // waiting send tries again later).
    let busy = {
        let mut asking = lock(&st.asking);
        let busy = asking.is_some();
        if !busy {
            *asking = Some(Asking {
                connection: id.to_string(),
                answer: answer_tx,
            });
        }
        busy
    };
    if busy {
        return incoming.decline(link).await;
    }
    let next = st
        .with_book(app, |b| b.next_folder(id))
        .ok()
        .flatten()
        .and_then(|NextFolder(f)| f.reopen().ok().map(|d| (f, d)));
    let folder = next.map(|(f, dest)| {
        let (name, path) = folder_name(&f.path);
        let view = FolderView {
            name,
            path,
            free_bytes: dest.available_bytes(),
        };
        *lock(&qs.folder) = Some(crate::quicksend::Held {
            path: f.path.clone(),
            identity: f.identity,
            dest,
        });
        view
    });
    let _ = app.emit(
        INCOMING_EVENT,
        KnownIncomingView {
            connection: id.to_string(),
            label: shown_safely(label),
            files: offered(files),
            bytes,
            folder,
        },
    );
    let answer = tokio::select! {
        a = answer_rx => a.ok(),
        () = tokio::time::sleep(pctwin_quicksend::ANSWER_WAIT) => None,
    };
    lock(&st.asking).take();
    let Some(Answer::Save { next_time }) = answer else {
        return incoming.decline(link).await;
    };
    let held = crate::quicksend::take_held(qs)
        .map_err(|_| pctwin_quicksend::QuickError::Destination("no folder was chosen".into()))?;
    *lock(&qs.received_into) = Some(crate::quicksend::Known {
        path: held.path.clone(),
        identity: held.identity,
    });
    if next_time
        && let Ok((folder, _)) = KnownFolder::choose(&held.path)
        && folder.identity == held.identity
    {
        let _ = st.with_book(app, |b| b.set_next_folder(id, NextFolder(folder)));
    }
    let held_path = held.path.clone();
    let reopen = crate::quicksend::reopened(&crate::quicksend::Known {
        path: held.path.clone(),
        identity: held.identity,
    });
    let (journal, me) = journal(app)?;
    *lock(&qs.live) = Some(live.clone());
    let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
    crate::quicksend::tick(app.clone(), live.clone(), done.clone());
    let r = incoming
        .accept_into(link, held.dest, None, &journal, &me, live)
        .await;
    done.store(true, std::sync::atomic::Ordering::Release);
    let r = r?;
    if live.control.stopped()
        && let Some(d) = reopen
    {
        remove_part_files(&d, &held_path, &paths);
    }
    let _ = app.emit(
        RECEIPT_EVENT,
        receipt_view("receiver", &r, live.control.stopped()),
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_engine_reason_has_its_own_refusal() {
        use pctwin_known::StoreError;
        assert_eq!(book_refusal(BookError::Restored), QsRefusal::Restored);
        assert_eq!(
            book_refusal(BookError::Store(StoreError::Unavailable)),
            QsRefusal::StoreUnavailable
        );
        assert_eq!(book_refusal(BookError::BadLabel), QsRefusal::BadName);
        assert_eq!(book_refusal(BookError::Full), QsRefusal::TooManyConnections);
        assert_eq!(book_refusal(BookError::NotFound), QsRefusal::NotFound);
        assert_eq!(
            folder_refusal(FolderProblem::HoldsKeys),
            QsRefusal::FolderHoldsKeys
        );
        assert_eq!(
            folder_refusal(FolderProblem::RunsAtSignIn),
            QsRefusal::FolderRunsAtSignIn
        );
        assert_eq!(
            folder_refusal(FolderProblem::NotTheSame),
            QsRefusal::FolderNotTheSame
        );
        assert_eq!(
            note_refusal(pctwin_quicksend::NoteError::Hidden),
            QsRefusal::NoteHidden
        );
        assert_eq!(
            waiting_refusal(&WaitingError::OverCap { needed: 1, room: 0 }),
            QsRefusal::OverCap
        );
        assert_eq!(waiting_refusal(&WaitingError::Days), QsRefusal::BadDays);
        assert_eq!(
            waiting_refusal(&WaitingError::CopyDamaged),
            QsRefusal::Failed
        );
    }

    /// Permission 3 delivers into a PCTwin-named folder inside the allowed one, made if missing;
    /// a link put at that name, or the allowed folder swapped, is refused.
    #[test]
    fn permission_3_delivers_only_into_its_own_plain_folder() {
        let d = tempfile::tempdir().unwrap();
        let allowed = d.path().join("From Ada");
        std::fs::create_dir_all(&allowed).unwrap();
        let (folder, _) = KnownFolder::choose(&allowed).unwrap();
        assert!(delivery_folder(&folder).is_ok());
        let inside = allowed.join(pctwin_known::book::DELIVERY_FOLDER);
        assert!(pctwin_known::folders::plain_folder(&inside));
        // A link at the PCTwin folder's name.
        std::fs::remove_dir(&inside).unwrap();
        let elsewhere = d.path().join("Startup");
        std::fs::create_dir_all(&elsewhere).unwrap();
        #[cfg(windows)]
        assert!(
            std::process::Command::new("cmd")
                .args(["/C", "mklink", "/J"])
                .arg(&inside)
                .arg(&elsewhere)
                .output()
                .unwrap()
                .status
                .success()
        );
        #[cfg(unix)]
        std::os::unix::fs::symlink(&elsewhere, &inside).unwrap();
        assert_eq!(
            delivery_folder(&folder).err(),
            Some(FolderProblem::NotPlain)
        );
        let _ = std::fs::remove_dir(&inside).or_else(|_| std::fs::remove_file(&inside));
        // The allowed folder swapped for another.
        std::fs::rename(&allowed, d.path().join("moved")).unwrap();
        std::fs::create_dir_all(&allowed).unwrap();
        assert_eq!(
            delivery_folder(&folder).err(),
            Some(FolderProblem::NotTheSame)
        );
    }

    #[test]
    fn a_connection_shows_its_folders_by_name_and_proves_nothing_about_a_person() {
        let v = view(pctwin_known::ConnectionInfo {
            id: "00".into(),
            label: "Ada\u{202E}kcab".into(),
            made_at: 1,
            last_used: 2,
            expires_at: 3,
            same_device: false,
            grant: None,
            next_folder: Some(PathBuf::from("/home/ada/Inbox")),
        });
        assert_eq!(v.label, "Ada[U+202E]kcab");
        assert!(!v.same_device);
        assert_eq!(v.next_folder.unwrap().name, "Inbox");
    }
}
