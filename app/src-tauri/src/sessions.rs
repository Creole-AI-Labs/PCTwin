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
//!   Cancel. A send picked up for a turn carries its Pause and Cancel from that moment.
//! - Nothing holds a session open: every wait races the session's End; a promised offer must
//!   arrive within [`OFFER_WAIT`]; a session quiet for [`IDLE_LIMIT`], older than [`MOST_AGE`], or
//!   whose last [`MOST_DECLINED`] offers in a row were declined, ends.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use pctwin_move::Live;
use pctwin_quicksend::Offer;
use pctwin_quicksend::session::{POLL_EVERY, Pace, SendState, Sends, Turn, Want, next_turn};
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

/// A promised offer (the other side took the turn) must begin arriving within this.
pub const OFFER_WAIT: Duration = Duration::from_secs(60);
/// A session with no offer either way for this long ends.
pub const IDLE_LIMIT: Duration = Duration::from_secs(15 * 60);
/// No session lasts longer than this.
pub const MOST_AGE: Duration = Duration::from_secs(2 * 60 * 60);
/// This many offers declined in a row end the session.
pub const MOST_DECLINED: u32 = 3;

/// One offer the other laptop made in an open session, received here: its result to tell
/// (`None`: told already by other events), and whether the person declined it.
pub(crate) struct Received {
    pub receipt: Option<ReceiptView>,
    pub declined: bool,
}

/// One open session.
struct Session {
    /// The remembered connection's name (`None`: the six-digit pairing; the screens name it).
    name: Option<String>,
    /// The remembered connection it is with.
    connection: Option<String>,
    /// The person chose End (or the connection was forgotten).
    ending: bool,
    /// Every wait in the session races this: `true` once it must end.
    end: tokio::sync::watch::Sender<bool>,
    opened: Instant,
    /// When an offer last went either way.
    last_offer: Instant,
    /// Offers declined in a row.
    declined: u32,
}

impl Session {
    /// Whether the session must end at `now`: End chosen, quiet too long, too old, or too many
    /// offers declined in a row.
    fn over(&self, now: Instant) -> bool {
        self.ending
            || now.saturating_duration_since(self.last_offer) >= IDLE_LIMIT
            || now.saturating_duration_since(self.opened) >= MOST_AGE
            || self.declined >= MOST_DECLINED
    }
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
        let now = Instant::now();
        lock(&self.sessions).insert(
            id.clone(),
            Session {
                name,
                connection,
                ending: false,
                end: tokio::sync::watch::Sender::new(false),
                opened: now,
                last_offer: now,
                declined: 0,
            },
        );
        self.told(app);
        id
    }

    /// Ends session `session`: marked, every wait in it woken, and its sends being sent stopped.
    fn end(&self, session: &str) {
        if let Some(s) = lock(&self.sessions).get_mut(session) {
            s.ending = true;
            s.end.send_replace(true);
        }
        let running: Vec<u64> = lock(&self.sends)
            .list()
            .into_iter()
            .filter(|q| q.session == session && q.state == SendState::Sending)
            .map(|q| q.id)
            .collect();
        for r in running {
            if let Some(l) = lock(&self.lives).get(&r) {
                l.control.stop();
            }
        }
        self.freed.notify_waiters();
    }

    /// What session `session`'s waits race: ends at once when the session is gone.
    fn ended(&self, session: &str) -> tokio::sync::watch::Receiver<bool> {
        match lock(&self.sessions).get(session) {
            Some(s) => s.end.subscribe(),
            None => tokio::sync::watch::Sender::new(true).subscribe(),
        }
    }

    /// Whether session `session` must end now (it is gone, or [`Session::over`]).
    fn over(&self, session: &str) -> bool {
        lock(&self.sessions)
            .get(session)
            .is_none_or(|s| s.over(Instant::now()))
    }

    /// An offer went in session `session`; `declined`: the person on the receiving side declined.
    fn offered(&self, session: &str, declined: bool) {
        if let Some(s) = lock(&self.sessions).get_mut(session) {
            s.last_offer = Instant::now();
            s.declined = if declined { s.declined + 1 } else { 0 };
        }
    }

    /// Picks up `session`'s next send for a turn: its Live made now, so a Pause or Cancel from
    /// this moment reaches it.
    fn pick_up(&self, session: &str) -> Option<(u64, Offer, Live)> {
        let mut sends = lock(&self.sends);
        let (id, offer) = sends.start(session)?;
        // A paused send is never picked up, so its Live starts unpaused; a Pause from now on
        // reaches it through `lives`.
        let live = Live::default();
        lock(&self.lives).insert(id, live.clone());
        Some((id, offer, live))
    }

    /// A send picked up for a turn that went to the other laptop: waiting again (still paused if
    /// paused), or gone if it was cancelled meanwhile.
    fn put_back(&self, id: u64, offer: Offer) {
        lock(&self.lives).remove(&id);
        lock(&self.sends).unstart(id, offer);
        self.freed.notify_waiters();
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
        // A session ending takes nothing more: said, never queued silently.
        if self.over(session) {
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
            self.end(&id);
        }
    }

    /// Sends `offer` (send `id` of `session`, picked up with `live`) over `link`: its result
    /// told, and it leaves the list. `show`: its progress is the one the send page shows. A send
    /// cancelled before it began is stopped at once (the other side is told nothing arrives).
    /// Whether the link is still usable.
    #[allow(clippy::too_many_arguments)]
    async fn send_one<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        session: &str,
        link: &mut pctwin_link::Link,
        id: u64,
        offer: Offer,
        live: Live,
        show: bool,
    ) -> bool {
        if lock(&self.sends)
            .marks(id)
            .is_some_and(|(_, cancelled)| cancelled)
        {
            live.control.stop();
        }
        let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
        if show {
            let qs = app.state::<AppQuickSend>();
            *lock(&qs.live) = Some(live.clone());
            crate::quicksend::tick(app.clone(), live.clone(), done.clone());
        }
        self.told(app);
        let sent = pctwin_quicksend::send_live(link, offer, &live).await;
        done.store(true, std::sync::atomic::Ordering::Release);
        lock(&self.lives).remove(&id);
        lock(&self.sends).finished(id);
        self.freed.notify_waiters();
        let ok = sent.is_ok();
        if let Ok(r) = sent {
            let declined = !r.lines.is_empty()
                && r.lines
                    .iter()
                    .all(|l| matches!(l.outcome, pctwin_quicksend::Outcome::Declined));
            self.offered(session, declined);
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
    /// to three laptops at once), then is sent over `link`, its progress shown. Whether it went
    /// and the session goes on.
    pub(crate) async fn first<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        session: &str,
        link: &mut pctwin_link::Link,
    ) -> bool {
        let mut ended = self.ended(session);
        loop {
            if self.over(session) {
                return false;
            }
            if let Some((id, offer, live)) = self.pick_up(session) {
                return self
                    .send_one(app, session, link, id, offer, live, true)
                    .await;
            }
            // Wait for a slot (or a Resume, a Cancel, an End), looking again now and then.
            tokio::select! {
                _ = tokio::time::timeout(POLL_EVERY, self.freed.notified()) => {}
                _ = ended.wait_for(|e| *e) => return false,
            }
        }
    }

    /// Takes turns in open session `session` over `link` until it ends: this laptop's queued
    /// offers when it is its turn, the other laptop's (received by `receive`, which asks where to
    /// save) when it is theirs. `leader`: this laptop made the first offer. Every wait races the
    /// session's End.
    pub(crate) async fn run<R: Runtime, F>(
        &self,
        app: &AppHandle<R>,
        session: &str,
        link: &mut pctwin_link::Link,
        leader: bool,
        mut receive: F,
    ) where
        F: AsyncFnMut(&mut pctwin_link::Link) -> Option<Received>,
    {
        let mut ended = self.ended(session);
        loop {
            // This side's next send is picked up when this side asks for a turn, so it is still
            // there if the turn is ours; put back if the turn is theirs.
            let mut taken: Option<(u64, Offer, Live)> = None;
            let turn = {
                let mut want = || {
                    if taken.is_none() {
                        taken = self.pick_up(session);
                    }
                    Want {
                        offer: taken.is_some(),
                        end: self.over(session),
                    }
                };
                tokio::select! {
                    t = next_turn(link, leader, &mut want, Pace::default()) => t,
                    _ = ended.wait_for(|e| *e) => Turn::Ended,
                }
            };
            if turn != Turn::Mine
                && let Some((id, offer, _)) = taken.take()
            {
                self.put_back(id, offer);
            }
            self.told(app);
            match turn {
                Turn::Mine => {
                    let Some((id, offer, live)) = taken else {
                        break;
                    };
                    if !self
                        .send_one(app, session, link, id, offer, live, false)
                        .await
                    {
                        break;
                    }
                }
                Turn::Theirs => {
                    let got = tokio::select! {
                        got = receive(link) => got,
                        _ = ended.wait_for(|e| *e) => None,
                    };
                    let Some(got) = got else { break };
                    self.offered(session, got.declined);
                    if let Some(receipt) = got.receipt {
                        let _ = app.emit(
                            SESSION_RECEIPT_EVENT,
                            SessionReceiptView {
                                session: session.to_string(),
                                send: None,
                                receipt,
                            },
                        );
                    }
                }
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
    if !lock(&sends.sessions).contains_key(&session) {
        return Err(QsRefusal::NotFound);
    }
    sends.end(&session);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn session(now: Instant) -> Session {
        Session {
            name: None,
            connection: None,
            ending: false,
            end: tokio::sync::watch::Sender::new(false),
            opened: now,
            last_offer: now,
            declined: 0,
        }
    }

    /// A session ends when End is chosen, after a quiet spell, when too old, or after too many
    /// declined offers in a row.
    #[test]
    fn a_session_ends_on_end_quiet_age_or_declines() {
        let t = Instant::now();
        let s = session(t);
        assert!(!s.over(t));
        assert!(!s.over(t + IDLE_LIMIT - Duration::from_secs(1)));
        assert!(s.over(t + IDLE_LIMIT), "quiet too long");
        let mut s = session(t);
        s.last_offer = t + MOST_AGE;
        assert!(s.over(t + MOST_AGE), "too old, however busy");
        let mut s = session(t);
        s.declined = MOST_DECLINED - 1;
        assert!(!s.over(t));
        s.declined = MOST_DECLINED;
        assert!(s.over(t), "declined too often");
        let mut s = session(t);
        s.ending = true;
        assert!(s.over(t));
    }

    /// A send picked up for a turn gets its Live at once: Cancel and Pause reach it, and a
    /// cancelled one never waits again.
    #[test]
    fn cancel_and_pause_reach_a_send_being_picked_up() {
        let o = OpenSends::default();
        let now = Instant::now();
        lock(&o.sessions).insert("s".into(), session(now));
        let offer = || {
            let d = tempfile::tempdir().unwrap();
            let f = d.path().join("a.txt");
            std::fs::write(&f, b"a").unwrap();
            let offer = Offer::of(&[f]).unwrap();
            (offer, d)
        };
        let (o1, _d1) = offer();
        let id = lock(&o.sends).add("s", 1, 1, o1);
        let (picked, offer1, live) = o.pick_up("s").unwrap();
        assert_eq!(picked, id);
        assert!(
            lock(&o.lives).contains_key(&id),
            "its Live exists from the pick-up"
        );
        // Pause while picked up: the Live is held, and it stays paused if put back.
        assert!(lock(&o.sends).set_paused(id, true));
        if let Some(l) = lock(&o.lives).get(&id) {
            l.control.pause();
        }
        assert!(live.control.paused());
        o.put_back(id, offer1);
        assert!(!lock(&o.sends).ready("s"), "still paused");
        assert!(lock(&o.sends).set_paused(id, false));
        // Cancel while picked up: stopped, and never waiting again.
        let (_, offer2, live) = o.pick_up("s").unwrap();
        assert_eq!(lock(&o.sends).cancel(id), Some(true));
        if let Some(l) = lock(&o.lives).get(&id) {
            l.control.stop();
        }
        assert!(live.control.stopped());
        o.put_back(id, offer2);
        assert!(lock(&o.sends).list().is_empty());
        // A paused send picked up later carries its Pause.
        let (o3, _d3) = offer();
        let three = lock(&o.sends).add("s", 1, 1, o3);
        let (_, offer3, live) = o.pick_up("s").unwrap();
        o.put_back(three, offer3);
        assert!(!live.control.paused());
        assert!(lock(&o.sends).set_paused(three, true));
        assert!(lock(&o.sends).set_paused(three, false));
        let _ = three;
    }

    /// A session that is ending takes no new send: refused, never queued silently.
    #[test]
    fn an_ending_session_takes_no_new_send() {
        let o = OpenSends::default();
        lock(&o.sessions).insert("s".into(), session(Instant::now()));
        o.end("s");
        assert!(o.over("s"));
        assert!(*o.ended("s").borrow(), "every wait in it wakes");
        assert!(
            *o.ended("gone").borrow(),
            "a session that is gone has ended"
        );
    }
}
