//! Open sends for the screens (Security Design Part T, "Add files and Send to", founder 29 October
//! 2026; the engine is `pctwin_quicksend::session`):
//!
//! - After the first offer of a six-digit exchange, or of a remembered connection, the session
//!   stays open: the sender's **Add files** and the receiver's **Send to <name>** each make a new
//!   offer in it ([`quicksend_session_add`]), the other side asked where to save exactly as for the
//!   first, each with its own result (`quicksend-session-receipt`). Either side can end it
//!   ([`quicksend_session_end`]); nothing arrives once it has ended.
//! - Several sends at once: one after another to the same laptop, up to three laptops at the same
//!   time; the rest wait their turn in the list (`quicksend-sends`), each with Pause, Resume and
//!   Cancel.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use pctwin_move::Live;
use pctwin_quicksend::Offer;
use pctwin_quicksend::session::{POLL_EVERY, SendState, Sends, Turn, Want, next_turn};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

use crate::quicksend::{AppQuickSend, ReceiptView, lock, receipt_view};
use crate::remembered::QsRefusal;

/// The open sessions changed: the whole list.
pub const SESSIONS_EVENT: &str = "quicksend-sessions";
/// The list of sends changed: the whole list.
pub const SENDS_EVENT: &str = "quicksend-sends";
/// An offer in an open session ended: its result.
pub const SESSION_RECEIPT_EVENT: &str = "quicksend-session-receipt";

/// One open session.
struct Session {
    /// The remembered connection's name (`None`: the six-digit pairing; the screens name it).
    name: Option<String>,
    /// The remembered connection it is with.
    connection: Option<String>,
    /// The person chose End.
    ending: bool,
}

#[derive(Default)]
pub struct OpenSends {
    sessions: Mutex<BTreeMap<String, Session>>,
    sends: Mutex<Sends<Offer>>,
    /// Each running send's progress and controls.
    lives: Mutex<BTreeMap<u64, Live>>,
    /// Wakes the openers of sessions to remembered connections (a slot came free).
    pub(crate) freed: tokio::sync::Notify,
}

pub type AppSends = Arc<OpenSends>;

pub fn new_state() -> AppSends {
    Arc::new(OpenSends::default())
}

/// An open session as the screens list it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionView {
    pub id: String,
    /// The other laptop's name for "Send to <name>", when it is a remembered connection.
    pub name: Option<String>,
    pub ending: bool,
}

/// A send in the list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendView {
    pub id: u64,
    pub session: String,
    pub name: Option<String>,
    pub files: u64,
    pub bytes: u64,
    pub state: SendState,
    pub paused: bool,
}

/// An offer in an open session that ended: whose it was (`send`: one of this laptop's, by id;
/// `None`: the other laptop's, received here) and how it came out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionReceiptView {
    pub session: String,
    pub send: Option<u64>,
    pub receipt: ReceiptView,
}

fn new_id() -> String {
    let mut raw = [0u8; 16];
    let _ = getrandom::fill(&mut raw);
    raw.iter().map(|b| format!("{b:02x}")).collect()
}

impl OpenSends {
    fn views(&self) -> Vec<SessionView> {
        lock(&self.sessions)
            .iter()
            .map(|(id, s)| SessionView {
                id: id.clone(),
                name: s.name.clone(),
                ending: s.ending,
            })
            .collect()
    }

    fn send_views(&self) -> Vec<SendView> {
        let sessions = lock(&self.sessions);
        lock(&self.sends)
            .list()
            .into_iter()
            .map(|q| SendView {
                name: sessions.get(&q.session).and_then(|s| s.name.clone()),
                id: q.id,
                session: q.session,
                files: q.files,
                bytes: q.bytes,
                state: q.state,
                paused: q.paused,
            })
            .collect()
    }

    fn told<R: Runtime>(&self, app: &AppHandle<R>) {
        let _ = app.emit(SESSIONS_EVENT, self.views());
        let _ = app.emit(SENDS_EVENT, self.send_views());
    }

    /// Opens a session (with remembered `connection` named `name`, or the six-digit pairing):
    /// its id.
    pub(crate) fn open<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        name: Option<String>,
        connection: Option<String>,
    ) -> String {
        let id = new_id();
        lock(&self.sessions).insert(
            id.clone(),
            Session {
                name,
                connection,
                ending: false,
            },
        );
        self.told(app);
        id
    }

    /// The open session with remembered connection `connection`, if any.
    pub(crate) fn with_connection(&self, connection: &str) -> Option<String> {
        lock(&self.sessions)
            .iter()
            .find(|(_, s)| s.connection.as_deref() == Some(connection))
            .map(|(id, _)| id.clone())
    }

    /// Queues `offer` into session `session`: its id.
    pub(crate) fn queue<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        session: &str,
        offer: Offer,
    ) -> Result<u64, QsRefusal> {
        if !lock(&self.sessions).contains_key(session) {
            return Err(QsRefusal::NotFound);
        }
        let files = offer.files();
        let bytes = files.iter().fold(0u64, |t, f| t.saturating_add(f.size));
        let id = lock(&self.sends).add(session, files.len() as u64, bytes, offer);
        self.told(app);
        Ok(id)
    }

    /// The session is over: its waiting sends go, never sent, and it leaves the list.
    pub(crate) fn close<R: Runtime>(&self, app: &AppHandle<R>, session: &str) {
        lock(&self.sends).session_ended(session);
        lock(&self.sessions).remove(session);
        self.freed.notify_waiters();
        self.told(app);
    }

    /// Ends the session with remembered connection `connection` (it was forgotten).
    pub(crate) fn end_connection(&self, connection: &str) {
        let ids: Vec<String> = lock(&self.sessions)
            .iter()
            .filter(|(_, s)| s.connection.as_deref() == Some(connection))
            .map(|(id, _)| id.clone())
            .collect();
        for id in ids {
            if let Some(s) = lock(&self.sessions).get_mut(&id) {
                s.ending = true;
            }
            let running: Vec<u64> = lock(&self.sends)
                .list()
                .into_iter()
                .filter(|q| q.session == id && q.state == SendState::Sending)
                .map(|q| q.id)
                .collect();
            for r in running {
                if let Some(l) = lock(&self.lives).get(&r) {
                    l.control.stop();
                }
            }
        }
    }

    /// Sends `offer` (send `id` of `session`, started) over `link`: its result told, and it leaves
    /// the list. Whether the link is still usable.
    async fn send_one<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        session: &str,
        link: &mut pctwin_link::Link,
        id: u64,
        offer: Offer,
    ) -> bool {
        let live = Live::default();
        lock(&self.lives).insert(id, live.clone());
        self.told(app);
        let sent = pctwin_quicksend::send_live(link, offer, &live).await;
        lock(&self.lives).remove(&id);
        lock(&self.sends).finished(id);
        self.freed.notify_waiters();
        let ok = sent.is_ok();
        if let Ok(r) = sent {
            let _ = app.emit(
                SESSION_RECEIPT_EVENT,
                SessionReceiptView {
                    session: session.to_string(),
                    send: Some(id),
                    receipt: receipt_view("sender", &r, live.control.stopped()),
                },
            );
        }
        self.told(app);
        ok
    }

    /// The first offer of a new session to a remembered laptop: waits its turn in the list (up
    /// to three laptops at once), then is sent over `link`. Whether it went and the session goes
    /// on.
    pub(crate) async fn first<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        session: &str,
        link: &mut pctwin_link::Link,
    ) -> bool {
        loop {
            if lock(&self.sessions).get(session).is_none_or(|s| s.ending) {
                return false;
            }
            let started = lock(&self.sends).start(session);
            if let Some((id, offer)) = started {
                return self.send_one(app, session, link, id, offer).await;
            }
            // Wait for a slot (or a Resume, a Cancel, an End), looking again now and then.
            let _ = tokio::time::timeout(POLL_EVERY, self.freed.notified()).await;
        }
    }

    /// Takes turns in open session `session` over `link` until it ends: this laptop's queued
    /// offers when it is its turn, the other laptop's (received by `receive`, which asks where to
    /// save) when it is theirs. `leader`: this laptop made the first offer.
    pub(crate) async fn run<R: Runtime, F>(
        &self,
        app: &AppHandle<R>,
        session: &str,
        link: &mut pctwin_link::Link,
        leader: bool,
        mut receive: F,
    ) where
        F: AsyncFnMut(&mut pctwin_link::Link) -> Option<Option<ReceiptView>>,
    {
        loop {
            // This side's next send is taken when this side asks for a turn, so it is still
            // there if the turn is ours; put back if the turn is theirs.
            let mut taken: Option<(u64, Offer)> = None;
            let turn = {
                let mut want = || {
                    if taken.is_none() {
                        taken = lock(&self.sends).start(session);
                    }
                    Want {
                        offer: taken.is_some(),
                        end: lock(&self.sessions).get(session).is_none_or(|s| s.ending),
                    }
                };
                next_turn(link, leader, &mut want, POLL_EVERY).await
            };
            if turn != Turn::Mine
                && let Some((id, offer)) = taken.take()
            {
                lock(&self.sends).unstart(id, offer);
            }
            self.told(app);
            match turn {
                Turn::Mine => {
                    let Some((id, offer)) = taken else { break };
                    if !self.send_one(app, session, link, id, offer).await {
                        break;
                    }
                }
                Turn::Theirs => match receive(link).await {
                    Some(Some(receipt)) => {
                        let _ = app.emit(
                            SESSION_RECEIPT_EVENT,
                            SessionReceiptView {
                                session: session.to_string(),
                                send: None,
                                receipt,
                            },
                        );
                    }
                    Some(None) => {}
                    None => break,
                },
                Turn::Ended => break,
            }
        }
        self.close(app, session);
    }
}

/// Add files (the sender) or Send to <name> (the receiver): the offer made on the pick page is
/// queued into open session `session`, sent when it is its turn (one after another to that
/// laptop, up to three laptops at once). The list of sends is returned.
#[tauri::command]
pub fn quicksend_session_add<R: Runtime>(
    app: AppHandle<R>,
    sends: State<'_, AppSends>,
    qs: State<'_, AppQuickSend>,
    session: String,
) -> Result<Vec<SendView>, QsRefusal> {
    let offer = lock(&qs.offer).take().ok_or(QsRefusal::NotWaiting)?;
    match sends.queue(&app, &session, offer) {
        Ok(_) => Ok(sends.send_views()),
        Err(e) => Err(e),
    }
}

/// Ends open session `session`: what is being sent now finishes or is stopped by the person;
/// nothing more goes or arrives, and sends still waiting for it leave the list.
#[tauri::command]
pub fn quicksend_session_end<R: Runtime>(
    app: AppHandle<R>,
    sends: State<'_, AppSends>,
    session: String,
) -> Result<(), QsRefusal> {
    let mut sessions = lock(&sends.sessions);
    let s = sessions.get_mut(&session).ok_or(QsRefusal::NotFound)?;
    s.ending = true;
    drop(sessions);
    sends.told(&app);
    Ok(())
}

#[tauri::command]
pub fn quicksend_sessions(sends: State<'_, AppSends>) -> Vec<SessionView> {
    sends.views()
}

#[tauri::command]
pub fn quicksend_sends(sends: State<'_, AppSends>) -> Vec<SendView> {
    sends.send_views()
}

/// Pause (`true`) or Resume a send in the list: a waiting one does not start; a running one is
/// held where it is.
fn set_paused<R: Runtime>(
    app: &AppHandle<R>,
    sends: &OpenSends,
    id: u64,
    paused: bool,
) -> Result<Vec<SendView>, QsRefusal> {
    if !lock(&sends.sends).set_paused(id, paused) {
        return Err(QsRefusal::NotFound);
    }
    if let Some(l) = lock(&sends.lives).get(&id) {
        if paused {
            l.control.pause();
        } else {
            l.control.carry_on();
        }
    }
    sends.freed.notify_waiters();
    sends.told(app);
    Ok(sends.send_views())
}

#[tauri::command]
pub fn quicksend_send_pause<R: Runtime>(
    app: AppHandle<R>,
    sends: State<'_, AppSends>,
    id: u64,
) -> Result<Vec<SendView>, QsRefusal> {
    set_paused(&app, &sends, id, true)
}

#[tauri::command]
pub fn quicksend_send_resume<R: Runtime>(
    app: AppHandle<R>,
    sends: State<'_, AppSends>,
    id: u64,
) -> Result<Vec<SendView>, QsRefusal> {
    set_paused(&app, &sends, id, false)
}

/// Cancel a send in the list: a waiting one leaves it; a running one is stopped (its receipt
/// says what arrived).
#[tauri::command]
pub fn quicksend_send_cancel<R: Runtime>(
    app: AppHandle<R>,
    sends: State<'_, AppSends>,
    id: u64,
) -> Result<Vec<SendView>, QsRefusal> {
    match lock(&sends.sends).cancel(id) {
        None => return Err(QsRefusal::NotFound),
        Some(true) => {
            if let Some(l) = lock(&sends.lives).get(&id) {
                l.control.stop();
            }
        }
        Some(false) => {}
    }
    sends.freed.notify_waiters();
    sends.told(&app);
    Ok(sends.send_views())
}

/// The state for the app's commands (for code outside this module).
pub(crate) fn of<R: Runtime>(app: &AppHandle<R>) -> AppSends {
    app.state::<AppSends>().inner().clone()
}
