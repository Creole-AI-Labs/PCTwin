//! An open send (Security Design Part T, "Add files and Send to", founder 29 October 2026): after
//! the first offer, the paired, authenticated link stays open, and either laptop may make another
//! offer inside it: the sender's "Add files", the receiver's "Send to <name>". Each is a whole new
//! offer ([`crate::send_live`] / [`crate::receive`]): the other side is asked where to save exactly
//! as for the first, and each has its own approval, fingerprints and result. Either side can only
//! offer; nothing here asks for or lists the other's files. Once the session ends (either side
//! says so, or the link goes) nothing more arrives.
//!
//! **Taking turns.** One link carries one offer at a time, and a read cannot be cancelled without
//! breaking the link, so the laptops take turns by a small fixed exchange: the leader (the laptop
//! that sent first) asks every [`POLL_EVERY`] whether either side has something to offer or wants
//! to end ([`next_turn`]); the follower answers. The leader's own offer goes first when both have
//! one; the other goes on the next turn. Both sides decide from the same two messages, so they
//! always agree whose turn it is.
//!
//! **Several sends at once** ([`Sends`]): one after another to the same laptop (queued in its open
//! session), up to [`MOST_AT_ONCE`] different laptops at the same time; the rest wait their turn
//! in a list, each with Pause and Cancel.

use std::time::Duration;

use pctwin_transfer::Channel;

/// A session's turn-taking message: `[SESSION][kind][flags]`.
const SESSION: u8 = 43;
const POLL: u8 = 1;
const ANSWER: u8 = 2;
/// This side has an offer to make.
const OFFERS: u8 = 1;
/// This side ends the session.
const ENDS: u8 = 2;
/// How often the leader asks, while nothing is being sent.
pub const POLL_EVERY: Duration = Duration::from_millis(500);
/// The follower answers no faster than this, whatever the leader does.
pub const MIN_POLL: Duration = Duration::from_millis(250);
/// The longest either side waits for the other's next turn-taking message: a silent laptop ends
/// the session.
pub const STEP_WAIT: Duration = Duration::from_secs(30);
/// Most laptops sent to at the same time; more wait their turn.
pub const MOST_AT_ONCE: usize = 3;

/// What this side wants at a turn.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Want {
    /// It has an offer ready.
    pub offer: bool,
    /// It ends the session.
    pub end: bool,
}

/// Whose turn it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Turn {
    /// This side makes its offer now ([`crate::send_live`]).
    Mine,
    /// The other side makes one: receive it ([`crate::receive`]), asking where to save.
    Theirs,
    /// The session is over: nothing more is sent or received.
    Ended,
}

fn message(kind: u8, w: Want) -> [u8; 3] {
    [
        SESSION,
        kind,
        (if w.offer { OFFERS } else { 0 }) | (if w.end { ENDS } else { 0 }),
    ]
}

/// The other side's turn-taking message of `kind`, read strictly.
fn read(bytes: &[u8], kind: u8) -> Option<Want> {
    match bytes {
        [SESSION, k, flags] if *k == kind && flags & !(OFFERS | ENDS) == 0 => Some(Want {
            offer: flags & OFFERS != 0,
            end: flags & ENDS != 0,
        }),
        _ => None,
    }
}

/// Who goes, from what each side wants: an end from either ends it; else the leader's offer, then
/// the follower's; `None`: nobody yet.
fn decide(leader: Want, follower: Want, i_lead: bool) -> Option<Turn> {
    if leader.end || follower.end {
        return Some(Turn::Ended);
    }
    let mine = |lead: bool| {
        if lead == i_lead {
            Turn::Mine
        } else {
            Turn::Theirs
        }
    };
    if leader.offer {
        Some(mine(true))
    } else if follower.offer {
        Some(mine(false))
    } else {
        None
    }
}

/// How turns are taken: how often the leader asks ([`POLL_EVERY`]), how fast the follower
/// answers at most ([`MIN_POLL`]), and how long either waits for the other ([`STEP_WAIT`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pace {
    pub every: Duration,
    pub min: Duration,
    pub step: Duration,
}

impl Default for Pace {
    fn default() -> Self {
        Self {
            every: POLL_EVERY,
            min: MIN_POLL,
            step: STEP_WAIT,
        }
    }
}

/// Waits for the next turn in an open session over `ch`. `leader`: this laptop sent first.
/// `want` is asked what this side wants each time (at most once per `pace.every` on the leader).
/// A message not there within `pace.step`, anything unexpected, or the link failing ends the
/// session; the follower never answers faster than `pace.min`, so a leader that asks too often
/// is slowed, not served.
pub async fn next_turn(
    ch: &mut impl Channel,
    leader: bool,
    want: &mut dyn FnMut() -> Want,
    pace: Pace,
) -> Turn {
    let mut answered: Option<tokio::time::Instant> = None;
    loop {
        let decided = if leader {
            let mine = want();
            if ch.send(&message(POLL, mine)).await.is_err() {
                return Turn::Ended;
            }
            let Some(theirs) = recv_within(ch, pace.step, ANSWER).await else {
                return Turn::Ended;
            };
            decide(mine, theirs, true)
        } else {
            let Some(theirs) = recv_within(ch, pace.step, POLL).await else {
                return Turn::Ended;
            };
            if let Some(last) = answered {
                tokio::time::sleep_until(last + pace.min).await;
            }
            let mine = want();
            if ch.send(&message(ANSWER, mine)).await.is_err() {
                return Turn::Ended;
            }
            answered = Some(tokio::time::Instant::now());
            decide(theirs, mine, false)
        };
        match decided {
            Some(turn) => return turn,
            None if leader => tokio::time::sleep(pace.every).await,
            None => {}
        }
    }
}

/// The other side's turn-taking message of `kind`, within `wait`.
async fn recv_within(ch: &mut impl Channel, wait: Duration, kind: u8) -> Option<Want> {
    let got = tokio::time::timeout(wait, ch.recv()).await.ok()?.ok()?;
    read(&got, kind)
}

/// Where a queued send is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SendState {
    /// Waiting its turn (its laptop busy, or [`MOST_AT_ONCE`] sends already running).
    Waiting,
    /// Being sent now.
    Sending,
}

/// One send in the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Queued {
    pub id: u64,
    /// The open session (one laptop) it goes to.
    pub session: String,
    pub files: u64,
    pub bytes: u64,
    pub state: SendState,
    /// Paused by the person: a waiting one does not start; a running one is held.
    pub paused: bool,
    /// Cancelled by the person while it was being picked up or sent: it never waits again.
    pub cancelled: bool,
}

/// Every send made and not yet finished, in the order made, with what each carries (`T`).
#[derive(Debug)]
pub struct Sends<T> {
    next: u64,
    items: Vec<(Queued, Option<T>)>,
}

impl<T> Default for Sends<T> {
    fn default() -> Self {
        Self {
            next: 1,
            items: Vec::new(),
        }
    }
}

impl<T> Sends<T> {
    /// Adds a send of `files` files and `bytes` bytes, carrying `what`, to `session`: its id.
    pub fn add(&mut self, session: &str, files: u64, bytes: u64, what: T) -> u64 {
        let id = self.next;
        self.next += 1;
        self.items.push((
            Queued {
                id,
                session: session.to_string(),
                files,
                bytes,
                state: SendState::Waiting,
                paused: false,
                cancelled: false,
            },
            Some(what),
        ));
        id
    }

    fn running(&self) -> usize {
        self.items
            .iter()
            .filter(|(q, _)| q.state == SendState::Sending)
            .count()
    }

    fn startable(&self, session: &str) -> Option<usize> {
        if self.running() >= MOST_AT_ONCE
            || self
                .items
                .iter()
                .any(|(q, _)| q.session == session && q.state == SendState::Sending)
        {
            return None;
        }
        self.items
            .iter()
            .position(|(q, _)| q.session == session && q.state == SendState::Waiting && !q.paused)
    }

    /// Whether `session` has a send that may start now: its laptop free, fewer than
    /// [`MOST_AT_ONCE`] running, and one waiting that is not paused.
    pub fn ready(&self, session: &str) -> bool {
        self.startable(session).is_some()
    }

    /// Starts `session`'s next send, when [`ready`](Self::ready): its id and what it carries.
    pub fn start(&mut self, session: &str) -> Option<(u64, T)> {
        let at = self.startable(session)?;
        let (q, what) = &mut self.items[at];
        q.state = SendState::Sending;
        Some((q.id, what.take()?))
    }

    /// A send started for a turn that went to the other laptop: waiting again, in its place (still
    /// paused if it was paused meanwhile), unless it was cancelled meanwhile, when it leaves the
    /// list. Whether it waits again.
    pub fn unstart(&mut self, id: u64, what: T) -> bool {
        let Some(at) = self.items.iter().position(|(q, _)| q.id == id) else {
            return false;
        };
        if self.items[at].0.cancelled {
            self.items.remove(at);
            return false;
        }
        let (q, slot) = &mut self.items[at];
        q.state = SendState::Waiting;
        *slot = Some(what);
        true
    }

    /// Whether send `id` is paused, and whether it was cancelled (for a send being picked up).
    pub fn marks(&self, id: u64) -> Option<(bool, bool)> {
        self.items
            .iter()
            .find(|(q, _)| q.id == id)
            .map(|(q, _)| (q.paused, q.cancelled))
    }

    /// Send `id` finished (sent, failed or stopped): it leaves the list.
    pub fn finished(&mut self, id: u64) {
        self.items.retain(|(q, _)| q.id != id);
    }

    /// Pauses (`true`) or resumes send `id`; whether it is in the list.
    pub fn set_paused(&mut self, id: u64, paused: bool) -> bool {
        match self.items.iter_mut().find(|(q, _)| q.id == id) {
            Some((q, _)) => {
                q.paused = paused;
                true
            }
            None => false,
        }
    }

    /// Cancels send `id`: a waiting one leaves the list (`Some(false)`); a running one stays until
    /// it has stopped (`Some(true)`: the caller stops it). `None`: not in the list.
    pub fn cancel(&mut self, id: u64) -> Option<bool> {
        let at = self.items.iter().position(|(q, _)| q.id == id)?;
        if self.items[at].0.state == SendState::Sending {
            self.items[at].0.cancelled = true;
            return Some(true);
        }
        self.items.remove(at);
        Some(false)
    }

    /// The session ended: its waiting sends leave the list, never sent (their ids).
    pub fn session_ended(&mut self, session: &str) -> Vec<u64> {
        let gone: Vec<u64> = self
            .items
            .iter()
            .filter(|(q, _)| q.session == session && q.state == SendState::Waiting)
            .map(|(q, _)| q.id)
            .collect();
        self.items.retain(|(q, _)| !gone.contains(&q.id));
        gone
    }

    /// The list, in the order made.
    pub fn list(&self) -> Vec<Queued> {
        self.items.iter().map(|(q, _)| q.clone()).collect()
    }

    /// The session a send goes to.
    pub fn session_of(&self, id: u64) -> Option<String> {
        self.items
            .iter()
            .find(|(q, _)| q.id == id)
            .map(|(q, _)| q.session.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turn_messages_are_read_strictly() {
        let w = Want {
            offer: true,
            end: false,
        };
        assert_eq!(read(&message(POLL, w), POLL), Some(w));
        assert_eq!(read(&message(POLL, w), ANSWER), None, "the wrong kind");
        assert_eq!(read(&[SESSION, POLL, 4], POLL), None, "an unknown flag");
        assert_eq!(read(&[SESSION, POLL], POLL), None);
        assert_eq!(read(&[SESSION, POLL, 0, 0], POLL), None);
        assert_eq!(read(&[40, POLL, 0], POLL), None);
    }

    #[test]
    fn both_sides_decide_the_same_turn() {
        let all = [
            Want::default(),
            Want {
                offer: true,
                end: false,
            },
            Want {
                offer: false,
                end: true,
            },
            Want {
                offer: true,
                end: true,
            },
        ];
        for l in all {
            for f in all {
                let lead = decide(l, f, true);
                let follow = decide(l, f, false);
                let flip = |t: Option<Turn>| {
                    t.map(|t| match t {
                        Turn::Mine => Turn::Theirs,
                        Turn::Theirs => Turn::Mine,
                        Turn::Ended => Turn::Ended,
                    })
                };
                assert_eq!(lead, flip(follow), "{l:?} {f:?}");
            }
        }
        let offer = Want {
            offer: true,
            end: false,
        };
        assert_eq!(
            decide(offer, offer, true),
            Some(Turn::Mine),
            "the leader first"
        );
        assert_eq!(decide(Want::default(), offer, true), Some(Turn::Theirs));
        assert_eq!(decide(Want::default(), Want::default(), true), None);
    }

    #[test]
    fn one_at_a_time_to_a_laptop_three_laptops_at_once() {
        let mut s: Sends<&str> = Sends::default();
        let a1 = s.add("a", 1, 10, "a1");
        let a2 = s.add("a", 2, 20, "a2");
        let b = s.add("b", 1, 1, "b");
        let c = s.add("c", 1, 1, "c");
        let d = s.add("d", 1, 1, "d");
        assert_eq!(s.start("a"), Some((a1, "a1")));
        s.unstart(a1, "a1");
        assert_eq!(s.start("a"), Some((a1, "a1")), "back in its place");
        assert!(!s.ready("a"), "one after another to the same laptop");
        assert_eq!(s.start("b"), Some((b, "b")));
        assert_eq!(s.start("c"), Some((c, "c")));
        assert!(!s.ready("d"), "three at once at most");
        assert_eq!(s.start("d"), None);
        s.finished(a1);
        assert_eq!(s.start("a"), Some((a2, "a2")));
        s.finished(b);
        assert!(s.ready("d"));
        assert_eq!(s.list().len(), 3);
        let _ = d;
    }

    /// A send picked up for a turn keeps the person's Cancel and Pause on its own entry: a
    /// cancelled one never waits again, a paused one is still paused when it waits again.
    #[test]
    fn cancel_and_pause_hold_for_a_send_being_picked_up() {
        let mut s: Sends<u8> = Sends::default();
        let two = s.add("a", 1, 1, 2);
        assert_eq!(s.start("a"), Some((two, 2)));
        assert_eq!(s.cancel(two), Some(true));
        assert_eq!(
            s.marks(two),
            Some((false, true)),
            "the mark stays on the send"
        );
        assert!(!s.unstart(two, 2), "a cancelled one never waits again");
        assert_eq!(s.marks(two), None);
        let three = s.add("a", 1, 1, 3);
        assert_eq!(s.start("a"), Some((three, 3)));
        assert!(s.set_paused(three, true));
        assert_eq!(s.marks(three), Some((true, false)));
        assert!(s.unstart(three, 3));
        assert!(!s.ready("a"), "paused while picked up: still paused");
        assert!(s.set_paused(three, false));
        assert_eq!(s.start("a"), Some((three, 3)));
    }

    #[test]
    fn pause_and_cancel_each() {
        let mut s: Sends<u8> = Sends::default();
        let one = s.add("a", 1, 1, 1);
        let two = s.add("a", 1, 1, 2);
        assert!(s.set_paused(one, true));
        assert_eq!(
            s.start("a"),
            Some((two, 2)),
            "a paused one waits, the next goes"
        );
        assert_eq!(
            s.cancel(two),
            Some(true),
            "running: stopped by the caller first"
        );
        assert_eq!(s.list().len(), 2);
        s.finished(two);
        assert!(!s.ready("a"), "still paused");
        assert!(s.set_paused(one, false));
        assert!(s.ready("a"));
        assert_eq!(s.cancel(one), Some(false));
        assert!(s.list().is_empty());
        assert_eq!(s.cancel(one), None);
        assert!(!s.set_paused(one, true));
        let three = s.add("b", 1, 1, 3);
        let four = s.add("c", 1, 1, 4);
        assert_eq!(s.session_ended("b"), vec![three]);
        assert_eq!(s.list().len(), 1, "another laptop's send stays");
        assert_eq!(s.session_of(four).as_deref(), Some("c"));
        s.finished(four);
        assert!(s.list().is_empty());
        assert_eq!(s.session_of(three), None);
    }
}
