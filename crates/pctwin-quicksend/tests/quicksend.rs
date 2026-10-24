//! Quick Send (Task List 1.9, expansion plan EX01): chosen files and folders to a laptop nearby,
//! over the same sealed connection and safety gate as a move. The sender offers names and sizes
//! only; the receiver accepts everything, some, or nothing, and chooses where they go; only what
//! it accepted is sent; nothing received is opened or run; both sides get a receipt saying what
//! arrived, where, and what did not.

use std::path::Path;
use std::time::Duration;

use pctwin_quicksend::{Incoming, Offer, Outcome, receive_offer, send};
use pctwin_transfer::{Channel, ChannelError};
use tokio::sync::mpsc;

/// How long a test waits for a step before calling it a deadlock. It is not a guess at the
/// runner's speed: every step here takes well under a second, so a busy runner never comes near
/// it, while a step that waits for ever still fails the test instead of hanging the run.
const HANG_AFTER: Duration = Duration::from_secs(300);

/// Runs `work` to its end, failing only if it is still waiting after [`HANG_AFTER`].
async fn no_hang<T>(work: impl std::future::Future<Output = T>) -> T {
    match tokio::time::timeout(HANG_AFTER, work).await {
        Ok(done) => done,
        Err(_) => panic!("hung: still waiting after {HANG_AFTER:?}, a deadlock, not a slow runner"),
    }
}

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
    no_hang(async {
        tokio::join!(async { send(&mut old, offer).await.unwrap() }, async {
            let incoming = receive_offer(&mut new).await.unwrap();
            decide(incoming, &mut new).await
        })
    })
    .await
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
            let v = pctwin_transfer::agree_version(&mut a, pctwin_transfer::MOVE_PROTOCOL)
                .await
                .unwrap_or(pctwin_transfer::PAGED_FROM);
            let _ = pctwin_transfer::send_inventory(&mut a, &record, v).await;
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
        let v = pctwin_transfer::agree_version(&mut a, pctwin_transfer::MOVE_PROTOCOL)
            .await
            .unwrap();
        pctwin_transfer::send_inventory(&mut a, &record, v)
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
    let (_, receipt) = no_hang(async { tokio::join!(greedy, receiving) }).await;
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
    let (sent, _) =
        no_hang(async { tokio::join!(async { send(&mut a, offer).await.unwrap() }, receiving) })
            .await;
    assert_eq!(b.starts, 1, "only the accepted letter was started");
    assert!(matches!(
        outcome(&sent, "letter.txt"),
        Outcome::Arrived { .. }
    ));
}

/// A sender that names another place for a file it offered (a startup folder) gets it refused:
/// where a file lands comes from the offer the receiver accepted, never from the sender's start.
#[tokio::test]
async fn a_sender_cannot_choose_where_a_file_lands() {
    let old = tempfile::tempdir().unwrap();
    let dest = tempfile::tempdir().unwrap();
    let jdir = tempfile::tempdir().unwrap();
    let offer = Offer::of(&chosen(old.path())).unwrap();
    let (record, mut jobs) = offer.into_parts();
    for job in &mut jobs {
        if job.path == "letter.txt" {
            job.path =
                "AppData/Roaming/Microsoft/Windows/Start Menu/Programs/Startup/run.cmd".into();
        }
    }
    let (mut a, mut b) = pair();
    let hostile = async {
        let v = pctwin_transfer::agree_version(&mut a, pctwin_transfer::MOVE_PROTOCOL)
            .await
            .unwrap();
        pctwin_transfer::send_inventory(&mut a, &record, v)
            .await
            .unwrap();
        let _answer = a.recv().await.unwrap();
        let mut sender = pctwin_transfer::SenderSession::new(jobs, 2);
        let _ = sender.run(&mut a).await;
    };
    let receiving = async {
        let incoming = receive_offer(&mut b).await.unwrap();
        incoming
            .accept(&mut b, dest.path(), None, &journal(jdir.path()), "1001")
            .await
            .unwrap()
    };
    let (_, receipt) = no_hang(async { tokio::join!(hostile, receiving) }).await;
    assert!(!dest.path().join("AppData").exists());
    assert!(!dest.path().join("letter.txt").exists());
    // The receipt keeps the typed reason, not only the words.
    assert!(matches!(
        outcome(&receipt, "letter.txt"),
        Outcome::Failed {
            cause: pctwin_transfer::Cause::RefusedBySafetyCheck,
            ..
        }
    ));
    // The files sent where the offer said still arrive.
    assert!(matches!(
        outcome(&receipt, "Holiday/beach.jpg"),
        Outcome::Arrived { .. }
    ));
}

/// A file that grew after it was offered is not copied, not even by one byte: the receiver
/// accepted what it was shown.
#[tokio::test]
async fn a_file_that_grew_after_it_was_offered_is_not_copied() {
    let old = tempfile::tempdir().unwrap();
    let dest = tempfile::tempdir().unwrap();
    let jdir = tempfile::tempdir().unwrap();
    let offer = Offer::of(&chosen(old.path())).unwrap();
    std::fs::write(old.path().join("letter.txt"), b"Dear Ada!").unwrap();
    let (sent, received) = run(offer, async |incoming: Incoming, ch: &mut Mem| {
        Some(
            incoming
                .accept(ch, dest.path(), None, &journal(jdir.path()), "1001")
                .await
                .unwrap(),
        )
    })
    .await;
    let received = received.unwrap();
    assert!(!dest.path().join("letter.txt").exists());
    assert!(
        matches!(outcome(&received, "letter.txt"), Outcome::Failed { cause: pctwin_transfer::Cause::ChangedDuringMove, why } if why.contains("changed since it was chosen")),
        "{:?}",
        outcome(&received, "letter.txt")
    );
    assert!(matches!(
        outcome(&sent, "letter.txt"),
        Outcome::Failed { .. }
    ));
    assert!(matches!(
        outcome(&received, "Holiday/beach.jpg"),
        Outcome::Arrived { .. }
    ));
}

/// Everything received comes from someone else's laptop, so the system treats each file as from
/// the internet, whatever the sender said (Security Design B). Linux has no standard marker.
#[cfg(any(windows, target_os = "macos"))]
#[tokio::test]
async fn every_received_file_is_marked_as_from_the_internet() {
    use pctwin_gate::{DownloadMark, download_mark_of};
    let old = tempfile::tempdir().unwrap();
    let dest = tempfile::tempdir().unwrap();
    let jdir = tempfile::tempdir().unwrap();
    let offer = Offer::of(&chosen(old.path())).unwrap();
    let _ = run(offer, async |incoming: Incoming, ch: &mut Mem| {
        Some(
            incoming
                .accept(ch, dest.path(), None, &journal(jdir.path()), "1001")
                .await
                .unwrap(),
        )
    })
    .await;
    for path in ["Holiday/beach.jpg", "Holiday/day 2/boat.jpg", "letter.txt"] {
        assert_eq!(
            download_mark_of(&dest.path().join(path)),
            Some(DownloadMark::Internet),
            "{path}"
        );
    }
}

/// A send the sender stopped before it began ends with a receipt on both sides: every accepted
/// file failed, none arrived, and nothing is written as if it had.
#[tokio::test]
async fn a_stopped_send_ends_with_a_receipt_and_nothing_is_called_arrived() {
    let old = tempfile::tempdir().unwrap();
    let new = tempfile::tempdir().unwrap();
    let offer = Offer::of(&chosen(old.path())).unwrap();
    let live = pctwin_move::Live::default();
    live.control.stop();
    let (mut a, mut b) = pair();
    let j = journal(new.path());
    let (sent, got) = no_hang(async {
        tokio::join!(
            async { pctwin_quicksend::send_live(&mut a, offer, &live).await },
            async {
                let incoming = receive_offer(&mut b).await.unwrap();
                incoming.accept(&mut b, new.path(), None, &j, "me").await
            }
        )
    })
    .await;
    let sent = sent.unwrap();
    assert_eq!(sent.lines.len(), 3);
    for line in &sent.lines {
        assert!(
            !matches!(
                line.outcome,
                Outcome::Arrived { .. } | Outcome::AlreadyThere { .. }
            ),
            "{line:?}"
        );
    }
    if let Ok(got) = got {
        for line in &got.lines {
            assert!(!matches!(line.outcome, Outcome::Arrived { .. }), "{line:?}");
        }
    }
    assert!(!new.path().join("letter.txt").exists());
}

/// A channel that counts the bytes it sends and, after `stop_after` messages, stops `live` (the
/// receiver's person pressing stop part way through).
struct Counting {
    inner: Mem,
    sent: std::sync::Arc<std::sync::atomic::AtomicU64>,
    messages: u64,
    stop_after: Option<(u64, pctwin_move::Live)>,
}

impl Channel for Counting {
    async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
        self.sent
            .fetch_add(data.len() as u64, std::sync::atomic::Ordering::SeqCst);
        self.messages += 1;
        if let Some((n, live)) = &self.stop_after
            && self.messages >= *n
        {
            live.control.stop();
        }
        self.inner.send(data).await
    }
    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        self.inner.recv().await
    }
}

/// Bytes that do not compress (a photo's), so what crosses is what is sent.
fn noise(len: usize, seed: u32) -> Vec<u8> {
    let mut x = seed.wrapping_mul(2_654_435_761) | 1;
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            (x >> 24) as u8
        })
        .collect()
}

/// "Send the missing pieces" (Task List; Q19): a send stopped part way is followed by an offer of
/// only what did not arrive, from the receipt. The receiver keeps what arrived, and a file it
/// holds partway is continued from its own record of the blocks it has: they are not sent again.
#[tokio::test]
async fn the_missing_pieces_are_offered_and_sent_once_more_and_nothing_else() {
    let old = tempfile::tempdir().unwrap();
    let new = tempfile::tempdir().unwrap();
    let big = noise(6 * 1024 * 1024, 7);
    write(&old.path().join("Holiday/beach.jpg"), &big);
    write(&old.path().join("letter.txt"), b"Dear Ada");
    let picked = vec![old.path().join("Holiday"), old.path().join("letter.txt")];
    let offer = Offer::of(&picked).unwrap();
    let laptop = offer.laptop();
    let j = journal(new.path());

    // The first send, stopped by the receiver's person part way through the big file.
    let stop = pctwin_move::Live::default();
    let (a, b) = pair();
    let (mut a, mut b) = (
        Counting {
            inner: a,
            sent: Default::default(),
            messages: 0,
            stop_after: None,
        },
        Counting {
            inner: b,
            sent: Default::default(),
            messages: 0,
            stop_after: Some((12, stop.clone())),
        },
    );
    let (first, _) = no_hang(async {
        tokio::join!(
            async { pctwin_quicksend::send(&mut a, offer).await.unwrap() },
            async {
                let incoming = receive_offer(&mut b).await.unwrap();
                incoming
                    .accept_live(&mut b, new.path(), None, &j, "me", &stop)
                    .await
            }
        )
    })
    .await;
    assert!(
        matches!(outcome(&first, "Holiday/beach.jpg"), Outcome::Failed { .. }),
        "{first:?}"
    );

    // The follow-up: only what is missing, from the receipt.
    let again = Offer::of_again(&picked, laptop)
        .unwrap()
        .only_missing(&first);
    let offered: Vec<String> = again.files().into_iter().map(|f| f.path).collect();
    let arrived_first = matches!(outcome(&first, "letter.txt"), Outcome::Arrived { .. });
    assert!(offered.contains(&"Holiday/beach.jpg".to_string()));
    assert_eq!(offered.contains(&"letter.txt".to_string()), !arrived_first);
    let (a, b) = pair();
    let mut a = Counting {
        inner: a,
        sent: Default::default(),
        messages: 0,
        stop_after: None,
    };
    let crossed = a.sent.clone();
    let mut b = Counting {
        inner: b,
        sent: Default::default(),
        messages: 0,
        stop_after: None,
    };
    let (second, got) = no_hang(async {
        tokio::join!(
            async { pctwin_quicksend::send(&mut a, again).await.unwrap() },
            async {
                let incoming = receive_offer(&mut b).await.unwrap();
                incoming
                    .accept(&mut b, new.path(), None, &j, "me")
                    .await
                    .unwrap()
            }
        )
    })
    .await;
    assert!(matches!(
        outcome(&second, "Holiday/beach.jpg"),
        Outcome::Arrived { .. }
    ));
    assert!(matches!(
        outcome(&got, "Holiday/beach.jpg"),
        Outcome::Arrived { .. }
    ));
    assert_eq!(
        std::fs::read(new.path().join("Holiday/beach.jpg")).unwrap(),
        big
    );
    // Blocks the receiver already had were not sent again.
    let sent_again = crossed.load(std::sync::atomic::Ordering::SeqCst);
    assert!(
        sent_again < big.len() as u64,
        "{sent_again} bytes crossed again for a file of {}",
        big.len()
    );
}

/// Everything under `dir`, as paths relative to it, sorted.
fn every_file(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut todo = vec![dir.to_path_buf()];
    while let Some(d) = todo.pop() {
        for e in std::fs::read_dir(&d).unwrap() {
            let e = e.unwrap();
            if e.file_type().unwrap().is_dir() {
                todo.push(e.path());
            } else {
                let rel = e
                    .path()
                    .strip_prefix(dir)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                out.push(rel);
            }
        }
    }
    out.sort();
    out
}

/// QS06: an archive, a program, a document with macros and a shortcut arrive exactly as sent:
/// nothing is unpacked, opened or run, and nothing appears beside them.
#[tokio::test]
async fn harmful_looking_files_arrive_as_they_are_and_nothing_is_unpacked_or_run() {
    let old = tempfile::tempdir().unwrap();
    let dest = tempfile::tempdir().unwrap();
    let jdir = tempfile::tempdir().unwrap();
    // A zip's own header and a tiny archive-bomb-like body; a Windows program's header; a
    // macro document; a Windows shortcut.
    let mut bomb = b"PK\x03\x04".to_vec();
    bomb.extend(std::iter::repeat_n(0u8, 4096));
    let things: Vec<(&str, Vec<u8>)> = vec![
        ("bomb.zip", bomb),
        ("setup.exe", b"MZ\x90\x00not really a program".to_vec()),
        (
            "report.docm",
            b"PK\x03\x04[Content_Types].xml vbaProject.bin".to_vec(),
        ),
        ("open me.lnk", b"L\x00\x00\x00\x01\x14\x02\x00".to_vec()),
    ];
    for (name, bytes) in &things {
        write(&old.path().join(name), bytes);
    }
    let picked: Vec<_> = things.iter().map(|(n, _)| old.path().join(n)).collect();
    let (_, received) = run(
        Offer::of(&picked).unwrap(),
        async |incoming: Incoming, ch: &mut Mem| {
            Some(
                incoming
                    .accept(ch, dest.path(), None, &journal(jdir.path()), "1001")
                    .await
                    .unwrap(),
            )
        },
    )
    .await;
    let received = received.unwrap();
    for (name, bytes) in &things {
        assert!(
            matches!(outcome(&received, name), Outcome::Arrived { .. }),
            "{name}"
        );
        assert_eq!(
            &std::fs::read(dest.path().join(name)).unwrap(),
            bytes,
            "{name} unchanged"
        );
    }
    let mut want: Vec<String> = things.iter().map(|(n, _)| (*n).to_string()).collect();
    want.sort();
    assert_eq!(
        every_file(dest.path()),
        want,
        "nothing unpacked or made beside them"
    );
}
