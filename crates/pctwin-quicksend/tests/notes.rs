//! A note or link over the sealed link (Quick Send Stage 1): the receiver gets it checked by the
//! same rules, nothing is written, and the sender hears only that it was taken.

use pctwin_quicksend::{Arrival, Note, NoteKind, Offer, QuickError, receive, send_note};
use pctwin_transfer::{Channel, ChannelError, MOVE_PROTOCOL, agree_version};
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

#[tokio::test]
async fn a_note_arrives_as_typed_and_the_sender_hears_it_was_taken() {
    let (mut a, mut b) = pair();
    let note = Note::new("Wi-Fi: Home\nPassword: 1234").unwrap();
    let (sent, got) = tokio::join!(send_note(&mut a, &note), receive(&mut b));
    sent.unwrap();
    match got.unwrap() {
        Arrival::Note(n) => {
            assert_eq!(n.text(), "Wi-Fi: Home\nPassword: 1234");
            assert_eq!(n.kind(), &NoteKind::Text);
        }
        Arrival::Files(_) => panic!("a note, not files"),
    }
}

#[tokio::test]
async fn a_link_arrives_as_a_link_with_its_site() {
    let (mut a, mut b) = pair();
    let note = Note::new("https://example.org/menu").unwrap();
    let (sent, got) = tokio::join!(send_note(&mut a, &note), receive(&mut b));
    sent.unwrap();
    let Ok(Arrival::Note(n)) = got else {
        panic!("a note")
    };
    assert_eq!(
        n.kind(),
        &NoteKind::Link {
            host: "example.org".into()
        }
    );
}

#[tokio::test]
async fn files_still_arrive_as_an_offer() {
    let old = tempfile::tempdir().unwrap();
    std::fs::write(old.path().join("a.txt"), b"hi").unwrap();
    let offer = Offer::of(&[old.path().join("a.txt")]).unwrap();
    let (mut a, mut b) = pair();
    let (sent, ()) = tokio::join!(pctwin_quicksend::send(&mut a, offer), async {
        let Ok(Arrival::Files(incoming)) = receive(&mut b).await else {
            panic!("files")
        };
        assert_eq!(incoming.files().len(), 1);
        incoming.decline(&mut b).await.unwrap();
    });
    sent.unwrap();
}

/// A sender that skips the rules: a note with a direction override, a tidied-looking one with a
/// space at its end, one too big, or one that is not text is refused and never taken.
#[tokio::test]
async fn a_note_that_breaks_the_rules_is_refused_on_arrival() {
    let mut big = vec![41u8];
    big.extend(std::iter::repeat_n(
        b'a',
        pctwin_quicksend::note::MAX_NOTE_BYTES + 1,
    ));
    for body in [
        [&[41u8][..], "invoice\u{202E}fdp.exe".as_bytes()].concat(),
        [&[41u8][..], b"hello "].concat(),
        vec![41, 0xFF, 0xFE],
        vec![41],
        big,
    ] {
        let (mut a, mut b) = pair();
        let sent = body.clone();
        let rogue = async move {
            agree_version(&mut a, MOVE_PROTOCOL).await.unwrap();
            a.send(&[40, 2]).await.unwrap();
            a.send(&sent).await.unwrap();
            // Nothing is taken: the receiver ends with an error and sends no answer.
            drop(a.tx);
            a.rx.recv().await
        };
        // The receiver's side ends with its answer, if any, so the rogue's wait ends too.
        let receiver = async move {
            let got = receive(&mut b).await;
            drop(b);
            got
        };
        let (answer, got) = tokio::join!(rogue, receiver);
        assert!(
            matches!(got, Err(QuickError::BadAnswer | QuickError::Note(_))),
            "{body:?}"
        );
        assert_eq!(answer, None, "never taken: {body:?}");
    }
}

/// Anything but files or a note said first is not a Quick Send.
#[tokio::test]
async fn an_exchange_that_says_neither_files_nor_a_note_is_refused() {
    for what in [vec![40, 3], vec![40], vec![40, 1, 0], vec![7, 1]] {
        let (mut a, mut b) = pair();
        let rogue = async {
            agree_version(&mut a, MOVE_PROTOCOL).await.unwrap();
            a.send(&what).await.unwrap();
        };
        let (_, got) = tokio::join!(rogue, receive(&mut b));
        assert!(matches!(got, Err(QuickError::NotAQuickSend)), "{what:?}");
    }
}

/// A sender is told only "taken", in its exact form.
#[tokio::test]
async fn the_sender_accepts_only_the_exact_taken_answer() {
    for answer in [vec![43], vec![42, 0], vec![]] {
        let (mut a, mut b) = pair();
        let note = Note::new("hi").unwrap();
        let rogue = async {
            agree_version(&mut b, MOVE_PROTOCOL).await.unwrap();
            let _what = b.recv().await.unwrap();
            let _note = b.recv().await.unwrap();
            b.send(&answer).await.unwrap();
        };
        let (sent, ()) = tokio::join!(send_note(&mut a, &note), rogue);
        assert!(matches!(sent, Err(QuickError::BadAnswer)), "{answer:?}");
    }
}
