//! Quick Send (Task List 1.9, expansion plan EX01): chosen files and folders to a laptop nearby,
//! over the same sealed connection and safety gate as a move. The sender offers names and sizes
//! only; the receiver accepts everything, some, or nothing, and chooses where they go; only what
//! it accepted is sent; nothing received is opened or run; both sides get a receipt saying what
//! arrived, where, and what did not.

use std::path::Path;

use pctwin_quicksend::{Incoming, Offer, Outcome, receive_offer, send};
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

fn write(path: &Path, bytes: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

/// A sender's chosen things: a folder with two photos inside, and a single document.
fn chosen(old: &Path) -> Vec<std::path::PathBuf> {
    write(&old.join("Holiday/beach.jpg"), b"sand and sea");
    write(&old.join("Holiday/day 2/boat.jpg"), b"a small boat");
    write(&old.join("letter.txt"), b"Dear Ada");
    vec![old.join("Holiday"), old.join("letter.txt")]
}

fn journal(dir: &Path) -> pctwin_journal::Journal {
    pctwin_journal::Journal::open(&dir.join("journal.redb")).unwrap()
}

/// Runs a send to completion: the receiver looks at the offer and does `decide` with it.
async fn run(
    offer: Offer,
    decide: impl AsyncFnOnce(Incoming, &mut Mem) -> Option<pctwin_quicksend::Receipt>,
) -> (pctwin_quicksend::Receipt, Option<pctwin_quicksend::Receipt>) {
    let (mut old, mut new) = pair();
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        tokio::join!(async { send(&mut old, offer).await.unwrap() }, async {
            let incoming = receive_offer(&mut new).await.unwrap();
            decide(incoming, &mut new).await
        })
    })
    .await
    .expect("the send finished")
}

fn outcome<'a>(receipt: &'a pctwin_quicksend::Receipt, path: &str) -> &'a Outcome {
    &receipt
        .lines
        .iter()
        .find(|l| l.path == path)
        .unwrap_or_else(|| panic!("{path} is on the receipt"))
        .outcome
}

#[tokio::test]
async fn chosen_files_and_folders_arrive_where_the_receiver_chose() {
    let old = tempfile::tempdir().unwrap();
    let dest = tempfile::tempdir().unwrap();
    let jdir = tempfile::tempdir().unwrap();
    let offer = Offer::of(&chosen(old.path())).unwrap();
    let (sent, received) = run(offer, async |incoming: Incoming, ch: &mut Mem| {
        let names: Vec<String> = incoming.files().into_iter().map(|f| f.path).collect();
        assert_eq!(
            names,
            ["Holiday/beach.jpg", "Holiday/day 2/boat.jpg", "letter.txt"]
        );
        assert_eq!(incoming.total_bytes(), 12 + 12 + 8);
        Some(
            incoming
                .accept(ch, dest.path(), None, &journal(jdir.path()), "1001")
                .await
                .unwrap(),
        )
    })
    .await;
    let received = received.unwrap();
    assert_eq!(
        std::fs::read(dest.path().join("Holiday/beach.jpg")).unwrap(),
        b"sand and sea"
    );
    assert_eq!(
        std::fs::read(dest.path().join("Holiday/day 2/boat.jpg")).unwrap(),
        b"a small boat"
    );
    assert_eq!(
        std::fs::read(dest.path().join("letter.txt")).unwrap(),
        b"Dear Ada"
    );
    for path in ["Holiday/beach.jpg", "Holiday/day 2/boat.jpg", "letter.txt"] {
        assert!(
            matches!(outcome(&sent, path), Outcome::Arrived { .. }),
            "{path}"
        );
        assert_eq!(
            outcome(&received, path),
            &Outcome::Arrived {
                landed_as: path.to_string()
            },
            "{path}"
        );
    }
    // The sender's files are untouched.
    assert_eq!(
        std::fs::read(old.path().join("letter.txt")).unwrap(),
        b"Dear Ada"
    );
}

#[tokio::test]
async fn only_what_the_receiver_picked_is_sent() {
    let old = tempfile::tempdir().unwrap();
    let dest = tempfile::tempdir().unwrap();
    let jdir = tempfile::tempdir().unwrap();
    let offer = Offer::of(&chosen(old.path())).unwrap();
    let (sent, received) = run(offer, async |incoming: Incoming, ch: &mut Mem| {
        let letter = incoming
            .files()
            .into_iter()
            .find(|f| f.path == "letter.txt")
            .unwrap()
            .id;
        Some(
            incoming
                .accept(
                    ch,
                    dest.path(),
                    Some(&[letter]),
                    &journal(jdir.path()),
                    "1001",
                )
                .await
                .unwrap(),
        )
    })
    .await;
    let received = received.unwrap();
    assert!(dest.path().join("letter.txt").exists());
    assert!(
        !dest.path().join("Holiday").exists(),
        "nothing not picked is written"
    );
    assert_eq!(outcome(&sent, "Holiday/beach.jpg"), &Outcome::Declined);
    assert_eq!(outcome(&received, "Holiday/beach.jpg"), &Outcome::Declined);
    assert!(matches!(
        outcome(&sent, "letter.txt"),
        Outcome::Arrived { .. }
    ));
}

#[tokio::test]
async fn a_declined_offer_writes_nothing_and_both_sides_know() {
    let old = tempfile::tempdir().unwrap();
    let dest = tempfile::tempdir().unwrap();
    let offer = Offer::of(&chosen(old.path())).unwrap();
    let (sent, received) = run(offer, async |incoming: Incoming, ch: &mut Mem| {
        incoming.decline(ch).await.unwrap();
        None
    })
    .await;
    assert!(received.is_none());
    assert!(std::fs::read_dir(dest.path()).unwrap().next().is_none());
    assert!(sent.lines.iter().all(|l| l.outcome == Outcome::Declined));
    assert_eq!(sent.lines.len(), 3);
}

#[tokio::test]
async fn a_name_already_taken_is_kept_and_the_new_one_lands_beside_it() {
    let old = tempfile::tempdir().unwrap();
    let dest = tempfile::tempdir().unwrap();
    let jdir = tempfile::tempdir().unwrap();
    write(&dest.path().join("letter.txt"), b"mine");
    let offer = Offer::of(&chosen(old.path())).unwrap();
    let (_, received) = run(offer, async |incoming: Incoming, ch: &mut Mem| {
        Some(
            incoming
                .accept(ch, dest.path(), None, &journal(jdir.path()), "1001")
                .await
                .unwrap(),
        )
    })
    .await;
    assert_eq!(
        std::fs::read(dest.path().join("letter.txt")).unwrap(),
        b"mine"
    );
    assert_eq!(
        outcome(&received.unwrap(), "letter.txt"),
        &Outcome::Arrived {
            landed_as: "letter (2).txt".into()
        }
    );
}

#[test]
fn nothing_chosen_is_not_an_offer() {
    assert!(Offer::of(&[]).is_err());
    let missing = tempfile::tempdir().unwrap();
    assert!(Offer::of(&[missing.path().join("not there.txt")]).is_err());
}

#[test]
fn two_chosen_things_with_one_name_are_kept_apart() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    write(&a.path().join("notes.txt"), b"one");
    write(&b.path().join("notes.txt"), b"two");
    let offer = Offer::of(&[a.path().join("notes.txt"), b.path().join("notes.txt")]).unwrap();
    let names: Vec<String> = offer.files().into_iter().map(|f| f.path).collect();
    // Listed in path order.
    assert_eq!(names, ["notes (2).txt", "notes.txt"]);
}

/// An offer holding a person's own things (not shared) is not a Quick Send: refused before the
/// person sees it.
#[tokio::test]
async fn an_offer_with_a_persons_things_is_refused() {
    let old = tempfile::tempdir().unwrap();
    let (mut record, _) = Offer::of(&chosen(old.path())).unwrap().into_parts();
    for item in &mut record.items {
        item.owner = pctwin_record::Owner::Person {
            account_id: "S-1-5-21-1001".into(),
        };
    }
    let (mut a, mut b) = pair();
    let (_, got) = tokio::join!(
        async {
            let _ = pctwin_transfer::agree_version(&mut a, pctwin_transfer::MOVE_PROTOCOL).await;
            let _ = pctwin_transfer::send_inventory(&mut a, &record).await;
        },
        receive_offer(&mut b)
    );
    assert!(
        matches!(got, Err(pctwin_quicksend::QuickError::NotAQuickSend)),
        "{:?}",
        got.err()
    );
}

/// A sender that ignores the answer and sends declined files too gets them refused by the
/// receiver: only what the person accepted is ever written.
#[tokio::test]
async fn declined_files_sent_anyway_are_refused_and_never_written() {
    let old = tempfile::tempdir().unwrap();
    let dest = tempfile::tempdir().unwrap();
    let jdir = tempfile::tempdir().unwrap();
    let offer = Offer::of(&chosen(old.path())).unwrap();
    let letter = offer
        .files()
        .into_iter()
        .find(|f| f.path == "letter.txt")
        .unwrap()
        .id;
    let (record, jobs) = offer.into_parts();
    let (mut a, mut b) = pair();
    let greedy = async {
        pctwin_transfer::agree_version(&mut a, pctwin_transfer::MOVE_PROTOCOL)
            .await
            .unwrap();
        pctwin_transfer::send_inventory(&mut a, &record)
            .await
            .unwrap();
        let _answer = a.recv().await.unwrap();
        let mut sender = pctwin_transfer::SenderSession::new(jobs, 2);
        let _ = sender.run(&mut a).await;
    };
    let receiving = async {
        let incoming = receive_offer(&mut b).await.unwrap();
        incoming
            .accept(
                &mut b,
                dest.path(),
                Some(&[letter]),
                &journal(jdir.path()),
                "1001",
            )
            .await
            .unwrap()
    };
    let (_, receipt) = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        tokio::join!(greedy, receiving)
    })
    .await
    .expect("the send finished");
    assert!(dest.path().join("letter.txt").exists());
    assert!(
        !dest.path().join("Holiday").exists(),
        "declined files are never written"
    );
    assert_eq!(outcome(&receipt, "Holiday/beach.jpg"), &Outcome::Declined);
}

/// The receiver's end of the connection, counting the files the sender starts.
struct CountingStarts {
    inner: Mem,
    starts: usize,
}

impl Channel for CountingStarts {
    async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
        self.inner.send(data).await
    }
    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        let m = self.inner.recv().await?;
        if matches!(
            pctwin_transfer::Message::decode(&m),
            Ok(pctwin_transfer::Message::StartFile { .. })
        ) {
            self.starts += 1;
        }
        Ok(m)
    }
}

/// Only the files the receiver accepted are even started: nothing declined crosses the network.
#[tokio::test]
async fn nothing_declined_is_even_started() {
    let old = tempfile::tempdir().unwrap();
    let dest = tempfile::tempdir().unwrap();
    let jdir = tempfile::tempdir().unwrap();
    let offer = Offer::of(&chosen(old.path())).unwrap();
    let letter = offer
        .files()
        .into_iter()
        .find(|f| f.path == "letter.txt")
        .unwrap()
        .id;
    let (mut a, b) = pair();
    let mut b = CountingStarts {
        inner: b,
        starts: 0,
    };
    let receiving = async {
        let incoming = receive_offer(&mut b).await.unwrap();
        incoming
            .accept(
                &mut b,
                dest.path(),
                Some(&[letter]),
                &journal(jdir.path()),
                "1001",
            )
            .await
            .unwrap()
    };
    let (sent, _) = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        tokio::join!(async { send(&mut a, offer).await.unwrap() }, receiving)
    })
    .await
    .expect("the send finished");
    assert_eq!(b.starts, 1, "only the accepted letter was started");
    assert!(matches!(
        outcome(&sent, "letter.txt"),
        Outcome::Arrived { .. }
    ));
}
