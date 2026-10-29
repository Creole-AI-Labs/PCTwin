//! Remembered connections (Security Design Part T): later sessions use Noise `KK` with the keys
//! both laptops remembered, fresh session keys each time, and nothing counts as authenticated on
//! the answering side until the starting side proved it is live.

use std::time::{Duration, Instant};

use pctwin_pairing::PairingError;
use pctwin_pairing::known::{KNOWN_TIMEOUT, StaticKeys, accept_known, check_public, open_known};
use zeroize::Zeroizing;

fn keys() -> (StaticKeys, StaticKeys) {
    (
        StaticKeys::generate().unwrap(),
        StaticKeys::generate().unwrap(),
    )
}

#[test]
fn two_laptops_that_remember_each_other_get_a_sealed_link() {
    let (a, b) = keys();
    let now = Instant::now();
    let (opening, m1) = open_known(&a, &b.public(), b"ctx", now).unwrap();
    let (accepting, m2) = accept_known(&b, &a.public(), b"ctx", &m1, now).unwrap();
    let (mut ta, m3) = opening.finish(&m2, now).unwrap();
    let mut tb = accepting.confirm(&m3, now).unwrap();
    let sealed = ta.seal(b"hello").unwrap();
    assert_eq!(tb.open(&sealed).unwrap().as_slice(), b"hello");
    let back = tb.seal(b"back").unwrap();
    assert_eq!(ta.open(&back).unwrap().as_slice(), b"back");
}

#[test]
fn each_session_has_fresh_keys() {
    let (a, b) = keys();
    let now = Instant::now();
    let run = || {
        let (o, m1) = open_known(&a, &b.public(), b"", now).unwrap();
        let (acc, m2) = accept_known(&b, &a.public(), b"", &m1, now).unwrap();
        let (mut t, m3) = o.finish(&m2, now).unwrap();
        acc.confirm(&m3, now).unwrap();
        (m1, t.seal(b"same").unwrap())
    };
    let (m1a, ca) = run();
    let (m1b, cb) = run();
    assert_ne!(m1a, m1b);
    assert_ne!(ca, cb);
}

#[test]
fn a_laptop_without_the_remembered_key_cannot_answer_or_start() {
    let (a, b) = keys();
    let mallory = StaticKeys::generate().unwrap();
    let now = Instant::now();
    // Mallory answers in B's place: A refuses message 2.
    let (opening, m1) = open_known(&a, &b.public(), b"", now).unwrap();
    assert!(accept_known(&mallory, &a.public(), b"", &m1, now).is_err());
    let (_, m2) = accept_known(
        &mallory,
        &a.public(),
        b"",
        &{ open_known(&a, &mallory.public(), b"", now).unwrap().1 },
        now,
    )
    .unwrap();
    assert_eq!(
        opening.finish(&m2, now).err(),
        Some(PairingError::HandshakeFailed)
    );
    // Mallory starts in A's place: B refuses message 1.
    let (_, m1) = open_known(&mallory, &b.public(), b"", now).unwrap();
    assert!(accept_known(&b, &a.public(), b"", &m1, now).is_err());
}

#[test]
fn a_different_context_does_not_meet() {
    let (a, b) = keys();
    let now = Instant::now();
    let (_, m1) = open_known(&a, &b.public(), b"one", now).unwrap();
    assert!(accept_known(&b, &a.public(), b"two", &m1, now).is_err());
}

/// A replayed message 1 gets an answer, but the replayer cannot read it or confirm: nothing is
/// authenticated on the answering side.
#[test]
fn a_replayed_first_message_never_becomes_a_link() {
    let (a, b) = keys();
    let now = Instant::now();
    let (opening, m1) = open_known(&a, &b.public(), b"", now).unwrap();
    let (accepting, m2) = accept_known(&b, &a.public(), b"", &m1, now).unwrap();
    let (_, m3) = opening.finish(&m2, now).unwrap();
    accepting.confirm(&m3, now).unwrap();
    // The replay: B answers again; the old confirmation does not fit the new session.
    let (again, _) = accept_known(&b, &a.public(), b"", &m1, now).unwrap();
    assert_eq!(
        again.confirm(&m3, now).err(),
        Some(PairingError::HandshakeFailed)
    );
}

#[test]
fn a_reflected_first_message_is_refused() {
    let (a, b) = keys();
    let now = Instant::now();
    let (_, m1) = open_known(&a, &b.public(), b"", now).unwrap();
    // A's own listener, thinking the message is from B.
    assert!(accept_known(&a, &b.public(), b"", &m1, now).is_err());
}

#[test]
fn late_messages_are_refused() {
    let (a, b) = keys();
    let now = Instant::now();
    let late = now + KNOWN_TIMEOUT + Duration::from_secs(1);
    let (opening, m1) = open_known(&a, &b.public(), b"", now).unwrap();
    let (accepting, m2) = accept_known(&b, &a.public(), b"", &m1, now).unwrap();
    let (_, m3) = open_known(&a, &b.public(), b"", now).unwrap();
    let _ = m3;
    assert_eq!(opening.finish(&m2, late).err(), Some(PairingError::Expired));
    assert_eq!(
        accepting.confirm(&[1, 14], late).err(),
        Some(PairingError::Expired)
    );
}

#[test]
fn malformed_messages_are_refused_without_panicking() {
    let (a, b) = keys();
    let now = Instant::now();
    let (_, m1) = open_known(&a, &b.public(), b"", now).unwrap();
    for bad in [
        vec![],
        vec![1],
        vec![2, 12],
        vec![1, 13],
        vec![1, 12, 0, 0],
        vec![0; 600],
    ] {
        assert!(
            accept_known(&b, &a.public(), b"", &bad, now).is_err(),
            "{bad:?}"
        );
    }
    let mut wrong_kind = m1.clone();
    wrong_kind[1] = 13;
    assert!(accept_known(&b, &a.public(), b"", &wrong_kind, now).is_err());
    assert_eq!(
        open_known(&a, &b.public(), &[0; 65], now).err(),
        Some(PairingError::TooLarge)
    );
}

#[test]
fn the_public_half_follows_from_the_private_half() {
    let a = StaticKeys::generate().unwrap();
    let again = StaticKeys::from_private(Zeroizing::new(**a.private()));
    assert_eq!(a.public(), again.public());
    assert_ne!(a.public(), StaticKeys::generate().unwrap().public());
}

#[test]
fn a_low_order_public_key_is_refused() {
    assert_eq!(check_public(&[0; 32]), Err(PairingError::InvalidKey));
    let mut one = [0u8; 32];
    one[0] = 1;
    assert_eq!(check_public(&one), Err(PairingError::InvalidKey));
    // Order 8 point (RFC 7748 test lists): e0eb7a7c3b41b8ae1656e3faf19fc46ada098deb9c32b1fd866205165f49b800
    let p8: [u8; 32] = [
        0xe0, 0xeb, 0x7a, 0x7c, 0x3b, 0x41, 0xb8, 0xae, 0x16, 0x56, 0xe3, 0xfa, 0xf1, 0x9f, 0xc4,
        0x6a, 0xda, 0x09, 0x8d, 0xeb, 0x9c, 0x32, 0xb1, 0xfd, 0x86, 0x62, 0x05, 0x16, 0x5f, 0x49,
        0xb8, 0x00,
    ];
    assert_eq!(check_public(&p8), Err(PairingError::InvalidKey));
    assert!(check_public(&StaticKeys::generate().unwrap().public()).is_ok());
    let (a, _) = keys();
    assert_eq!(
        open_known(&a, &[0; 32], b"", Instant::now()).err(),
        Some(PairingError::InvalidKey)
    );
}
