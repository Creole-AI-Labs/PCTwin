//! The move protocol version handshake (Task List 1.1): right after the number match, before the
//! old laptop's list, both laptops say the oldest and newest move protocol they speak, over the
//! sealed link (so nobody in between can talk them down to an older one), and both use the
//! highest version they share. If they share none, each laptop says which of the two is older, so
//! the person knows which one to update. What the other laptop sends is checked like anything
//! else from a stranger: one short message in exactly the agreed form, or the move stops.

use pctwin_transfer::{
    Channel, ChannelError, MOVE_PROTOCOL, TransferError, VERSION_WAIT, VersionGap, Versions,
    agree_version, choose_version,
};
use std::future::Future;
use std::time::Duration;

use proptest::prelude::*;
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

fn v(oldest: u16, newest: u16) -> Versions {
    Versions::new(oldest, newest).unwrap()
}

/// Waits for `f`, failing the test instead of hanging if it never finishes.
async fn within<F: Future>(f: F) -> F::Output {
    tokio::time::timeout(Duration::from_secs(5), f)
        .await
        .expect("the handshake hung")
}

/// Runs the handshake between two laptops speaking `a` and `b`.
async fn agree(
    a: Versions,
    b: Versions,
) -> (Result<u16, TransferError>, Result<u16, TransferError>) {
    let (mut x, mut y) = pair();
    within(async { tokio::join!(agree_version(&mut x, a), agree_version(&mut y, b)) }).await
}

/// Has this laptop send `bytes` as its version message and returns what the other side made of it.
async fn receive(bytes: &[u8]) -> Result<u16, TransferError> {
    let (mut x, mut y) = pair();
    x.send(bytes).await.unwrap();
    within(agree_version(&mut y, MOVE_PROTOCOL)).await
}

/// The version message a laptop speaking `ours` sends (the other side echoes it back).
async fn sent_by(ours: Versions) -> Vec<u8> {
    let (mut x, mut y) = pair();
    let (_, got) = within(async {
        tokio::join!(agree_version(&mut x, ours), async {
            let got = y.recv().await.unwrap();
            y.send(&got).await.unwrap();
            got
        })
    })
    .await;
    got
}

#[test]
fn this_release_speaks_version_two() {
    assert_eq!(MOVE_PROTOCOL.oldest(), 2);
    assert_eq!(MOVE_PROTOCOL.newest(), 2);
}

#[test]
fn a_range_must_start_at_one_or_later_and_not_run_backwards() {
    assert!(Versions::new(0, 1).is_none());
    assert!(Versions::new(0, 0).is_none());
    assert!(Versions::new(3, 2).is_none());
    assert!(Versions::new(2, 2).is_some());
    assert!(Versions::new(1, u16::MAX).is_some());
}

#[tokio::test]
async fn two_laptops_on_this_release_agree_on_version_two() {
    let (a, b) = agree(MOVE_PROTOCOL, MOVE_PROTOCOL).await;
    assert_eq!(a.unwrap(), 2);
    assert_eq!(b.unwrap(), 2);
}

#[tokio::test]
async fn both_use_the_highest_version_they_share() {
    // A newer laptop that still speaks one version back, and an older one.
    let (a, b) = agree(v(1, 2), v(1, 1)).await;
    assert_eq!(a.unwrap(), 1);
    assert_eq!(b.unwrap(), 1);
    let (a, b) = agree(v(2, 5), v(1, 3)).await;
    assert_eq!(a.unwrap(), 3);
    assert_eq!(b.unwrap(), 3);
}

#[tokio::test]
async fn each_laptop_says_which_one_is_older_when_they_share_no_version() {
    let (newer, older) = agree(v(3, 4), v(1, 2)).await;
    assert!(
        matches!(
            newer,
            Err(TransferError::VersionGap(VersionGap::OtherLaptopOlder))
        ),
        "{newer:?}"
    );
    assert!(
        matches!(
            older,
            Err(TransferError::VersionGap(VersionGap::ThisLaptopOlder))
        ),
        "{older:?}"
    );
}

#[tokio::test]
async fn the_version_message_is_short_and_sent_once() {
    let (mut x, mut y) = pair();
    let (a, _) = within(async {
        tokio::join!(agree_version(&mut x, v(2, 7)), async {
            let got = y.recv().await.unwrap();
            y.send(&got).await.unwrap();
            got
        })
    })
    .await;
    assert_eq!(a.unwrap(), 7);
    drop(x);
    // Nothing else was sent after the one message.
    assert!(y.recv().await.is_err());
}

#[tokio::test]
async fn a_message_of_the_wrong_kind_or_size_is_refused() {
    let good = sent_by(MOVE_PROTOCOL).await;
    assert!(receive(&good).await.is_ok());

    let mut wrong_kind = good.clone();
    wrong_kind[0] ^= 0xff;
    let mut short = good.clone();
    short.pop();
    let mut long = good.clone();
    long.push(0);
    for bad in [wrong_kind, short, long, Vec::new()] {
        let got = receive(&bad).await;
        assert!(
            matches!(got, Err(TransferError::Protocol(_))),
            "{bad:?}: {got:?}"
        );
    }
}

#[tokio::test]
async fn a_range_that_starts_at_zero_or_runs_backwards_is_refused() {
    let good = sent_by(v(1, 1)).await;
    let kind = good[0];
    for (oldest, newest) in [(0u16, 1u16), (0, 0), (2, 1)] {
        let mut bad = vec![kind];
        bad.extend_from_slice(&oldest.to_be_bytes());
        bad.extend_from_slice(&newest.to_be_bytes());
        let got = receive(&bad).await;
        assert!(
            matches!(got, Err(TransferError::Protocol(_))),
            "{oldest}..{newest}: {got:?}"
        );
    }
}

/// Every release reads this message the same way: a kind byte, then the oldest and the newest
/// version as big-endian 16-bit numbers.
#[tokio::test]
async fn the_message_form_is_fixed_for_every_release() {
    let kind = sent_by(v(1, 1)).await[0];
    // Apart from the transfer's kinds (1 to 15) and the inventory's (32, 33).
    assert_eq!(kind, 34);
    assert_eq!(sent_by(v(2, 0x0103)).await, vec![kind, 0, 2, 1, 3]);
    // A later laptop that speaks versions 1 to 3 still agrees with this one on 2.
    assert_eq!(receive(&[kind, 0, 1, 0, 3]).await.unwrap(), 2);
}

#[tokio::test]
async fn a_dropped_connection_is_reported_as_one() {
    let (mut x, y) = pair();
    drop(y);
    let got = within(agree_version(&mut x, MOVE_PROTOCOL)).await;
    assert!(
        matches!(got, Err(TransferError::ConnectionDropped)),
        "{got:?}"
    );
}

#[tokio::test]
async fn a_connection_that_drops_before_the_reply_is_reported_as_one() {
    let (mut x, y) = pair();
    // The other laptop takes the message, then hangs up without answering.
    let Mem { tx, rx: _kept } = y;
    drop(tx);
    let got = within(agree_version(&mut x, MOVE_PROTOCOL)).await;
    assert!(
        matches!(got, Err(TransferError::ConnectionDropped)),
        "{got:?}"
    );
}

fn versions() -> impl Strategy<Value = Versions> {
    (1u16..=40, 0u16..=40).prop_map(|(oldest, extra)| v(oldest, oldest.saturating_add(extra)))
}

proptest! {
    /// The choice is the highest version in both ranges, the same whichever laptop computes it,
    /// and a gap is reported from both sides with the older laptop named the same way.
    #[test]
    fn the_choice_is_the_highest_shared_version_from_either_side(a in versions(), b in versions()) {
        let shared: Vec<u16> = (a.oldest()..=a.newest())
            .filter(|n| (b.oldest()..=b.newest()).contains(n))
            .collect();
        let ab = choose_version(a, b);
        let ba = choose_version(b, a);
        match shared.last() {
            Some(&best) => {
                prop_assert_eq!(ab, Ok(best));
                prop_assert_eq!(ba, Ok(best));
            }
            None => {
                let a_older = a.newest() < b.oldest();
                let (ab_gap, ba_gap) = if a_older {
                    (VersionGap::ThisLaptopOlder, VersionGap::OtherLaptopOlder)
                } else {
                    (VersionGap::OtherLaptopOlder, VersionGap::ThisLaptopOlder)
                };
                prop_assert_eq!(ab, Err(ab_gap));
                prop_assert_eq!(ba, Err(ba_gap));
            }
        }
    }

    /// Whatever arrives, the handshake returns instead of crashing, and accepts only a version
    /// both laptops speak.
    #[test]
    fn any_bytes_end_in_an_answer_never_a_crash(bytes in proptest::collection::vec(any::<u8>(), 0..12)) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build().unwrap();
        if let Ok(n) = rt.block_on(receive(&bytes)) {
            prop_assert!(n >= MOVE_PROTOCOL.oldest() && n <= MOVE_PROTOCOL.newest());
        }
    }
}

/// A laptop that connects and then never answers is given up on, instead of leaving the other
/// waiting for ever; an answer that comes just in time is taken.
#[tokio::test(start_paused = true)]
async fn a_laptop_that_never_answers_is_given_up_on() {
    let (mut x, _y) = pair();
    let t = tokio::time::Instant::now();
    let got = agree_version(&mut x, MOVE_PROTOCOL).await;
    assert!(matches!(got, Err(TransferError::NoAnswer(_))), "{got:?}");
    assert!(t.elapsed() >= VERSION_WAIT, "{:?}", t.elapsed());
    assert!(t.elapsed() < VERSION_WAIT + Duration::from_secs(1));

    let (mut x, mut y) = pair();
    let (a, b) = tokio::join!(agree_version(&mut x, MOVE_PROTOCOL), async {
        tokio::time::sleep(VERSION_WAIT - Duration::from_secs(1)).await;
        agree_version(&mut y, MOVE_PROTOCOL).await
    });
    assert_eq!(a.unwrap(), 2);
    assert_eq!(b.unwrap(), 2);
}
