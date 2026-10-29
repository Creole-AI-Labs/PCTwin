//! An open send: Add files and Send to <name> (Security Design Part T, founder 29 October 2026).

use std::path::{Path, PathBuf};
use std::time::Duration;

use pctwin_quicksend::session::{Pace, Turn, Want, next_turn};
use pctwin_quicksend::{Arrival, Offer, Outcome, receive, send_live};
use pctwin_transfer::{Channel, ChannelError};
use tokio::sync::mpsc;

struct Mem {
    tx: mpsc::UnboundedSender<Vec<u8>>,
    rx: mpsc::UnboundedReceiver<Vec<u8>>,
}

impl Channel for Mem {
    async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
        self.tx.send(data.to_vec()).map_err(|_| ChannelError)
    }
    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        self.rx.recv().await.ok_or(ChannelError)
    }
}

fn pair() -> (Mem, Mem) {
    let (a_tx, b_rx) = mpsc::unbounded_channel();
    let (b_tx, a_rx) = mpsc::unbounded_channel();
    (Mem { tx: a_tx, rx: a_rx }, Mem { tx: b_tx, rx: b_rx })
}

const EVERY: Pace = Pace {
    every: Duration::from_millis(5),
    min: Duration::from_millis(1),
    step: Duration::from_secs(5),
};

fn file(dir: &Path, name: &str, body: &[u8]) -> Vec<PathBuf> {
    let p = dir.join(name);
    std::fs::write(&p, body).unwrap();
    vec![p]
}

fn journal(dir: &Path) -> pctwin_journal::Journal {
    pctwin_journal::Journal::open(&dir.join("journal.redb")).unwrap()
}

/// The other side receives one offer into `dest`, as asked where to save.
async fn take(ch: &mut Mem, dest: &Path, jdir: &Path) -> Vec<String> {
    let Ok(Arrival::Files(incoming)) = receive(ch).await else {
        panic!("an offer")
    };
    let names: Vec<String> = incoming.files().into_iter().map(|f| f.path).collect();
    incoming
        .accept(ch, dest, None, &journal(jdir), "1001")
        .await
        .unwrap();
    names
}

async fn turn(ch: &mut Mem, leader: bool, offer: bool, end: bool) -> Turn {
    next_turn(ch, leader, &mut move || Want { offer, end }, EVERY).await
}

async fn give(
    ch: &mut Mem,
    offer: Offer,
) -> Result<pctwin_quicksend::Receipt, pctwin_quicksend::QuickError> {
    send_live(ch, offer, &pctwin_move::Live::default()).await
}

/// After the first offer, the sender adds files, then the receiver sends some back, each a new
/// offer with its own result, each received where the other side chose; then the session ends.
#[tokio::test]
async fn add_files_and_send_to_are_new_offers_in_the_same_session() {
    let (a_dir, b_dir, a_in, b_in, ja, jb) = (dirs(), dirs(), dirs(), dirs(), dirs(), dirs());
    let (mut a, mut b) = pair();
    // The first offer, A to B.
    let first = Offer::of(&file(a_dir.path(), "one.txt", b"one")).unwrap();
    let (sent, names) = tokio::join!(give(&mut a, first), take(&mut b, b_in.path(), jb.path()));
    assert!(matches!(
        sent.unwrap().lines[0].outcome,
        Outcome::Arrived { .. }
    ));
    assert_eq!(names, ["one.txt"]);
    // Add files: A (the leader) has another offer; B has nothing.
    let (ta, tb) = tokio::join!(
        turn(&mut a, true, true, false),
        turn(&mut b, false, false, false)
    );
    assert_eq!((ta, tb), (Turn::Mine, Turn::Theirs));
    let more = Offer::of(&file(a_dir.path(), "two.txt", b"two")).unwrap();
    let (sent, names) = tokio::join!(give(&mut a, more), take(&mut b, b_in.path(), jb.path()));
    assert!(sent.is_ok());
    assert_eq!(names, ["two.txt"]);
    // Send to <name>: B offers back; A is asked where to save, like any offer.
    let (ta, tb) = tokio::join!(
        turn(&mut a, true, false, false),
        turn(&mut b, false, true, false)
    );
    assert_eq!((ta, tb), (Turn::Theirs, Turn::Mine));
    let back = Offer::of(&file(b_dir.path(), "reply.txt", b"reply")).unwrap();
    let (names, sent) = tokio::join!(take(&mut a, a_in.path(), ja.path()), give(&mut b, back));
    assert!(sent.is_ok());
    assert_eq!(names, ["reply.txt"]);
    assert_eq!(
        std::fs::read(a_in.path().join("reply.txt")).unwrap(),
        b"reply"
    );
    assert_eq!(std::fs::read(b_in.path().join("two.txt")).unwrap(), b"two");
    // Either side ends it: both agree.
    let (ta, tb) = tokio::join!(
        turn(&mut a, true, false, false),
        turn(&mut b, false, true, true)
    );
    assert_eq!((ta, tb), (Turn::Ended, Turn::Ended));
}

/// Both have an offer ready at once: the leader's goes first, the other's on the next turn; and
/// while neither has one, the turns wait.
#[tokio::test]
async fn two_offers_at_once_go_one_after_the_other() {
    let (mut a, mut b) = pair();
    let mut b_asked = 0;
    let mut b_want = || {
        b_asked += 1;
        Want {
            offer: b_asked > 3,
            end: false,
        }
    };
    let (ta, tb) = tokio::join!(
        turn(&mut a, true, false, false),
        next_turn(&mut b, false, &mut b_want, EVERY)
    );
    assert_eq!(
        (ta, tb),
        (Turn::Theirs, Turn::Mine),
        "after a few quiet turns"
    );
    let (ta, tb) = tokio::join!(
        turn(&mut a, true, true, false),
        turn(&mut b, false, true, false)
    );
    assert_eq!((ta, tb), (Turn::Mine, Turn::Theirs), "the leader first");
}

/// Once the link is gone, or the other side says anything out of turn, the session is over:
/// nothing more arrives.
#[tokio::test]
async fn nothing_arrives_once_the_session_ended() {
    let (mut a, b) = pair();
    drop(b);
    assert_eq!(turn(&mut a, true, true, false).await, Turn::Ended);
    let (a, mut b) = pair();
    drop(a);
    assert_eq!(turn(&mut b, false, true, false).await, Turn::Ended);
    // An offer started out of turn (no poll first) is not taken as one.
    let (mut a, mut b) = pair();
    let rogue = async {
        a.send(&[40, 1]).await.unwrap();
        a
    };
    let (_a, turn) = tokio::join!(rogue, turn(&mut b, false, false, false));
    assert_eq!(turn, Turn::Ended);
}

fn dirs() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

/// A laptop that goes silent in the middle of turn-taking ends the session after a while, on
/// either side: nothing holds a session open forever.
#[tokio::test(start_paused = true)]
async fn a_silent_laptop_ends_the_session() {
    let pace = Pace {
        every: Duration::from_millis(500),
        min: Duration::from_millis(250),
        step: Duration::from_secs(30),
    };
    // The follower hears nothing from the leader.
    let (_a, mut b) = pair();
    let start = tokio::time::Instant::now();
    let t = next_turn(&mut b, false, &mut || Want::default(), pace).await;
    assert_eq!(t, Turn::Ended);
    assert!(start.elapsed() >= Duration::from_secs(30));
    // The leader gets no answer.
    let (mut a, _b) = pair();
    let t = next_turn(&mut a, true, &mut || Want::default(), pace).await;
    assert_eq!(t, Turn::Ended);
}

/// A leader that asks as fast as it can is answered no faster than the follower's floor.
#[tokio::test(start_paused = true)]
async fn the_follower_answers_no_faster_than_its_floor() {
    let (mut a, mut b) = pair();
    let min = Duration::from_millis(250);
    let fast = Pace {
        every: Duration::ZERO,
        min,
        step: Duration::from_secs(30),
    };
    let mut answers = 0;
    let mut quiet = || {
        answers += 1;
        Want {
            offer: false,
            end: answers > 8,
        }
    };
    let start = tokio::time::Instant::now();
    let mut nothing = Want::default;
    let (ta, tb) = tokio::join!(
        next_turn(&mut a, true, &mut nothing, fast),
        next_turn(&mut b, false, &mut quiet, fast)
    );
    assert_eq!((ta, tb), (Turn::Ended, Turn::Ended));
    assert!(start.elapsed() >= min * 8, "{:?}", start.elapsed());
}
