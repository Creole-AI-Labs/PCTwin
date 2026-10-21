//! The link test after pairing: measured on version 4 and later, skipped (no time given) with an
//! older laptop, and anything but the test refused.

use pctwin_transfer::{
    Channel, ChannelError, PROBE_SINCE, TransferError, answer_link_test, measure_link,
};
use tokio::sync::mpsc;

struct Mem {
    tx: mpsc::UnboundedSender<Vec<u8>>,
    rx: mpsc::UnboundedReceiver<Vec<u8>>,
}

fn mem_pair() -> (Mem, Mem) {
    let (a_tx, b_rx) = mpsc::unbounded_channel();
    let (b_tx, a_rx) = mpsc::unbounded_channel();
    (Mem { tx: a_tx, rx: a_rx }, Mem { tx: b_tx, rx: b_rx })
}

impl Channel for Mem {
    async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
        self.tx.send(data.to_vec()).map_err(|_| ChannelError)
    }
    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        self.rx.recv().await.ok_or(ChannelError)
    }
}

#[tokio::test]
async fn both_laptops_on_version_four_measure_the_link() {
    let (mut old, mut new) = mem_pair();
    let (answered, measured) = tokio::join!(
        answer_link_test(&mut old, PROBE_SINCE),
        measure_link(&mut new, PROBE_SINCE)
    );
    answered.unwrap();
    assert!(measured.unwrap().is_some_and(|speed| speed > 0));
}

#[tokio::test]
async fn with_an_older_laptop_nothing_is_said_and_no_speed_is_given() {
    let (mut old, mut new) = mem_pair();
    answer_link_test(&mut old, PROBE_SINCE - 1).await.unwrap();
    assert_eq!(measure_link(&mut new, PROBE_SINCE - 1).await.unwrap(), None);
    // Nothing crossed either way.
    drop(old);
    assert!(new.recv().await.is_err());
}

#[tokio::test]
async fn an_answer_that_is_not_the_test_is_refused() {
    let (mut old, mut new) = mem_pair();
    let script = async {
        // The old laptop answers the ask with something else.
        old.recv().await.unwrap();
        old.send(&[37, 1, 2, 3]).await.unwrap();
        old
    };
    let (_old, measured) = tokio::join!(script, measure_link(&mut new, PROBE_SINCE));
    assert!(
        matches!(measured, Err(TransferError::Protocol(_))),
        "{measured:?}"
    );
}

#[tokio::test]
async fn the_old_laptop_answers_only_the_ask() {
    let (mut old, mut new) = mem_pair();
    new.send(&[1, 2, 3]).await.unwrap();
    let answered = answer_link_test(&mut old, PROBE_SINCE).await;
    assert!(
        matches!(answered, Err(TransferError::Protocol(_))),
        "{answered:?}"
    );
}
