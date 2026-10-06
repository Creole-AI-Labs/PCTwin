//! Regression tests for the findings of the fresh-context security review (Security Design Part J).
//! F1: the sender must check the number the person picked.
//! F2: failed attempts are budgeted: pauses, then a lockout until the person starts again.
//! F4: junk messages must not cancel a pairing in progress; waiting for confirmation times out.
//! F9: degenerate SPAKE2 points are rejected.

use curve25519_dalek::constants::EIGHT_TORSION;
use curve25519_dalek::edwards::CompressedEdwardsY;
use hkdf::Hkdf;
use pctwin_pairing::{
    CONFIRMATION_TIMEOUT, MATCH_NUMBER_RANGE, MAX_FAILED_ATTEMPTS, PairingCode, PairingError,
    ReceiverSession, RotatingSender, SenderAwaitingConfirmation, SenderSession, SenderStatus,
};
use sha2::Sha256;
use spake2::{Ed25519Group, Identity, Password, Spake2};
use std::time::{Duration, Instant};

// ---------- F1: a hand-built receiver that skips the human comparison ----------

/// An attacker who knows the code (a lucky guess or a glance at the screen) reimplements the
/// receiver from the public protocol, without PCTwin's `choose()` and its check.
fn raw_receiver(
    sender: &mut RotatingSender,
    now: Instant,
) -> (SenderAwaitingConfirmation, snow::TransportState) {
    let code = sender.code().unwrap();
    let m1 = sender.message_1().unwrap().to_vec();
    let id = m1[2..10].to_vec();
    let spake_a = m1[10..43].to_vec();
    let (st, spake_b) = Spake2::<Ed25519Group>::start_b(
        &Password::new(code.as_bytes()),
        &Identity::new(b"pctwin/v1/sender"),
        &Identity::new(b"pctwin/v1/receiver"),
    );
    let key = st.finish(&spake_a).unwrap();
    let mut psk = [0u8; 32];
    Hkdf::<Sha256>::new(Some(b"pctwin/v1/pairing-psk"), &key)
        .expand(b"noise psk", &mut psk)
        .unwrap();
    let mut prologue = b"pctwin/v1/pairing-prologue".to_vec();
    prologue.push(1);
    prologue.extend(&id);
    prologue.extend(&spake_a);
    prologue.extend(&spake_b);
    let mut hs = snow::Builder::new("Noise_NNpsk0_25519_ChaChaPoly_BLAKE2s".parse().unwrap())
        .psk(0, &psk)
        .unwrap()
        .prologue(&prologue)
        .unwrap()
        .build_initiator()
        .unwrap();
    let mut buf = [0u8; 128];
    let n = hs.write_message(&[], &mut buf).unwrap();
    let mut m2 = vec![1u8, 2];
    m2.extend(&id);
    m2.extend(&spake_b);
    m2.extend(&buf[..n]);
    let (waiting, m3) = sender.receive(&m2, now).unwrap();
    hs.read_message(&m3[2..], &mut buf).unwrap();
    (waiting, hs.into_transport_mode().unwrap())
}

fn sealed_confirmation(tr: &mut snow::TransportState, plain: &[u8]) -> Vec<u8> {
    let mut out = [0u8; 64];
    let k = tr.write_message(plain, &mut out).unwrap();
    let mut m4 = vec![1u8, 4];
    m4.extend(&out[..k]);
    m4
}

#[test]
fn control_the_hand_built_receiver_works_when_it_knows_the_number() {
    // Proves the attack harness is a faithful implementation, so the failures below are real.
    let now = Instant::now();
    let mut sender = RotatingSender::new(now).unwrap();
    let (waiting, mut tr) = raw_receiver(&mut sender, now);
    let mut plain = b"pctwin/v1/confirmed".to_vec();
    plain.push(waiting.match_number());
    let m4 = sealed_confirmation(&mut tr, &plain);
    assert!(waiting.receive_confirmation(&m4, now).is_ok());
}

#[test]
fn a_code_holder_who_never_saw_the_number_cannot_confirm() {
    let now = Instant::now();
    let mut sender = RotatingSender::new(now).unwrap();
    let (waiting, mut tr) = raw_receiver(&mut sender, now);
    let guess = MATCH_NUMBER_RANGE
        .clone()
        .find(|&n| n != waiting.match_number())
        .unwrap();
    let mut plain = b"pctwin/v1/confirmed".to_vec();
    plain.push(guess);
    let m4 = sealed_confirmation(&mut tr, &plain);
    let rejected = waiting.receive_confirmation(&m4, now).unwrap_err();
    assert_eq!(rejected.error(), PairingError::WrongNumber);
    assert!(
        rejected.into_retry().is_none(),
        "a wrong number ends the pairing"
    );
}

#[test]
fn a_confirmation_without_a_number_is_rejected() {
    // The pre-fix message format: just the label, no pick.
    let now = Instant::now();
    let mut sender = RotatingSender::new(now).unwrap();
    let (waiting, mut tr) = raw_receiver(&mut sender, now);
    let m4 = sealed_confirmation(&mut tr, b"pctwin/v1/confirmed");
    let rejected = waiting.receive_confirmation(&m4, now).unwrap_err();
    assert!(rejected.into_retry().is_none());
}

// ---------- F2: failure budget ----------

fn burn_one(sender: &mut RotatingSender, now: Instant) -> PairingError {
    let code = sender.code().unwrap();
    let msg1 = sender.message_1().unwrap().to_vec();
    let wrong = if code == "000000" { "000001" } else { "000000" };
    let (_r, msg2) = ReceiverSession::respond(&PairingCode::parse(wrong).unwrap(), &msg1).unwrap();
    sender.receive(&msg2, now).unwrap_err()
}

#[test]
fn five_failures_lock_pairing_until_the_person_starts_again() {
    let mut t = Instant::now();
    let mut sender = RotatingSender::new(t).unwrap();
    for i in 1..=MAX_FAILED_ATTEMPTS {
        burn_one(&mut sender, t);
        assert_eq!(sender.failed_attempts(), i);
        t += Duration::from_secs(60);
        sender.tick(t).unwrap();
    }
    assert_eq!(sender.status(), SenderStatus::Locked);
    assert!(sender.code().is_none() && sender.message_1().is_none());

    // Time alone does not unlock it.
    sender.tick(t + Duration::from_secs(3600)).unwrap();
    assert_eq!(sender.status(), SenderStatus::Locked);

    // The person chooses to start again: a fresh code that works.
    sender.unlock(t).unwrap();
    assert_eq!(sender.failed_attempts(), 0);
    let code = sender.code().unwrap();
    let msg1 = sender.message_1().unwrap().to_vec();
    let (_r, msg2) = ReceiverSession::respond(&PairingCode::parse(&code).unwrap(), &msg1).unwrap();
    assert!(sender.receive(&msg2, t).is_ok());
}

#[test]
fn replies_while_locked_or_cooling_down_are_refused_without_counting() {
    let now = Instant::now();
    let mut sender = RotatingSender::new(now).unwrap();
    let stale = sender.message_1().unwrap().to_vec();
    burn_one(&mut sender, now);
    assert_eq!(sender.failed_attempts(), 1);

    // During the one-second pause, any reply is refused and nothing more is counted.
    let (_r, msg2) =
        ReceiverSession::respond(&PairingCode::parse("123456").unwrap(), &stale).unwrap();
    assert!(matches!(
        sender.receive(&msg2, now + Duration::from_millis(500)),
        Err(PairingError::CoolingDown)
    ));
    assert_eq!(sender.failed_attempts(), 1);
}

#[test]
fn an_online_guesser_gets_at_most_five_tries_before_a_person_must_act() {
    let mut t = Instant::now();
    let mut sender = RotatingSender::new(t).unwrap();
    let mut tries = 0;
    for _ in 0..1000 {
        sender.tick(t).unwrap();
        if let Some(msg1) = sender.message_1().map(<[u8]>::to_vec) {
            let (_r, msg2) =
                ReceiverSession::respond(&PairingCode::parse("000000").unwrap(), &msg1).unwrap();
            if sender.receive(&msg2, t).is_err() {
                tries += 1;
            }
        }
        t += Duration::from_secs(1);
    }
    assert!(tries <= MAX_FAILED_ATTEMPTS, "made {tries} guesses");
    assert_eq!(sender.status(), SenderStatus::Locked);
}

#[test]
fn a_successful_pairing_resets_the_failure_count() {
    let mut t = Instant::now();
    let mut sender = RotatingSender::new(t).unwrap();
    burn_one(&mut sender, t);
    t += Duration::from_secs(1);
    sender.tick(t).unwrap();
    let code = sender.code().unwrap();
    let msg1 = sender.message_1().unwrap().to_vec();
    let (_r, msg2) = ReceiverSession::respond(&PairingCode::parse(&code).unwrap(), &msg1).unwrap();
    sender.receive(&msg2, t).unwrap();
    assert_eq!(sender.failed_attempts(), 0);
}

#[test]
fn replies_naming_unknown_codes_never_count_as_failures() {
    let now = Instant::now();
    let mut sender = RotatingSender::new(now).unwrap();
    let other = RotatingSender::new(now).unwrap();
    let foreign = other.message_1().unwrap().to_vec();
    for _ in 0..20 {
        let (_r, msg2) =
            ReceiverSession::respond(&PairingCode::parse("111111").unwrap(), &foreign).unwrap();
        assert!(matches!(
            sender.receive(&msg2, now),
            Err(PairingError::UnknownSession)
        ));
    }
    assert_eq!(sender.failed_attempts(), 0);
}

// ---------- F4: junk does not cancel; confirmation times out ----------

fn up_to_confirmation(now: Instant) -> (SenderAwaitingConfirmation, Vec<u8>) {
    let code = PairingCode::generate().unwrap();
    let (s, m1) = SenderSession::start(&code, now);
    let (r, m2) = ReceiverSession::respond(&code.clone(), &m1).unwrap();
    let (waiting, m3) = s.receive(&m2, now).unwrap();
    let choosing = r.receive(&m3).unwrap();
    let (_p, m4) = choosing.choose(waiting.match_number()).unwrap();
    (waiting, m4)
}

#[test]
fn junk_confirmations_do_not_cancel_the_wait() {
    let now = Instant::now();
    let (mut waiting, m4) = up_to_confirmation(now);
    let junk: Vec<Vec<u8>> = vec![
        vec![],
        vec![1],
        b"hello".to_vec(),
        vec![9, 4, 0, 0],
        vec![1, 3, 0, 0],
        {
            let mut forged = m4.clone();
            let last = forged.len() - 1;
            forged[last] ^= 1;
            forged
        },
        vec![1, 4].into_iter().chain([0xaa; 40]).collect(),
    ];
    for j in junk {
        let rejected = waiting.receive_confirmation(&j, now).unwrap_err();
        waiting = rejected
            .into_retry()
            .expect("junk must hand the waiting state back");
    }
    assert!(waiting.receive_confirmation(&m4, now).is_ok());
}

#[test]
fn confirmation_after_the_deadline_is_refused() {
    let now = Instant::now();
    let (waiting, m4) = up_to_confirmation(now);
    assert_eq!(waiting.deadline(), now + CONFIRMATION_TIMEOUT);
    let late = now + CONFIRMATION_TIMEOUT + Duration::from_millis(1);
    let rejected = waiting.receive_confirmation(&m4, late).unwrap_err();
    assert_eq!(rejected.error(), PairingError::Expired);
    assert!(rejected.into_retry().is_none());
}

#[test]
fn a_malformed_message_3_does_not_cancel_the_receiver() {
    let now = Instant::now();
    let code = PairingCode::generate().unwrap();
    let (s, m1) = SenderSession::start(&code, now);
    let (r, m2) = ReceiverSession::respond(&code.clone(), &m1).unwrap();
    let (_waiting, m3) = s.receive(&m2, now).unwrap();

    let mut r = r;
    for junk in [vec![], vec![1u8], vec![1, 4, 1, 2, 3], vec![7, 3, 0]] {
        r = r
            .receive(&junk)
            .unwrap_err()
            .into_retry()
            .expect("a malformed message 3 must hand the receiver back");
    }
    assert!(r.receive(&m3).is_ok());
}

// ---------- F9: degenerate SPAKE2 points ----------

const IDENTITY: [u8; 32] = {
    let mut b = [0u8; 32];
    b[0] = 1;
    b
};

fn with_point(msg: &[u8], offset: usize, point: [u8; 32]) -> Vec<u8> {
    let mut m = msg.to_vec();
    m[offset..offset + 32].copy_from_slice(&point);
    m
}

fn small_order_points() -> Vec<[u8; 32]> {
    EIGHT_TORSION
        .iter()
        .map(|p| p.compress().to_bytes())
        .collect()
}

#[test]
fn receiver_rejects_degenerate_points_from_the_sender() {
    let now = Instant::now();
    let code = PairingCode::generate().unwrap();
    let (_s, m1) = SenderSession::start(&code, now);
    // Message 1 layout: version, kind, 8-byte session tag, 'A', 32-byte point.
    let mut bad = small_order_points();
    bad.push(IDENTITY);
    // A real point with a small-order component mixed in.
    let honest = CompressedEdwardsY(m1[11..43].try_into().unwrap())
        .decompress()
        .unwrap();
    bad.push((honest + EIGHT_TORSION[1]).compress().to_bytes());
    for p in bad {
        assert!(
            ReceiverSession::respond(&code, &with_point(&m1, 11, p)).is_err(),
            "accepted point {p:02x?}"
        );
    }
}

#[test]
fn sender_rejects_degenerate_points_from_the_receiver() {
    let now = Instant::now();
    let code = PairingCode::generate().unwrap();
    // Message 2 layout: version, kind, 8-byte session tag, 'B', 32-byte point, Noise message.
    for p in small_order_points() {
        let (s, m1) = SenderSession::start(&code, now);
        let (_r, m2) = ReceiverSession::respond(&code.clone(), &m1).unwrap();
        assert!(s.receive(&with_point(&m2, 11, p), now).is_err());
    }
}
