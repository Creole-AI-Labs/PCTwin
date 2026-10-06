//! Regression tests for the findings of the fresh-context security review (Security Design Part J).
//! F1: only a person's pick on the sender (old laptop) can unlock it; approvals are checked.
//! F2: failed attempts are budgeted: pauses, then a lockout until the person starts again.
//! F4: junk messages must not cancel a pairing in progress; waiting times out.
//! F9: degenerate SPAKE2 points are rejected.

use curve25519_dalek::constants::EIGHT_TORSION;
use curve25519_dalek::edwards::CompressedEdwardsY;
use hkdf::Hkdf;
use pctwin_pairing::{
    CONFIRMATION_TIMEOUT, MATCH_NUMBER_RANGE, MAX_FAILED_ATTEMPTS, PairingCode, PairingError,
    ReceiverAwaitingApproval, ReceiverSession, RotatingSender, SenderChoosing, SenderSession,
    SenderStatus,
};
use sha2::Sha256;
use spake2::{Ed25519Group, Identity, Password, Spake2};
use std::time::{Duration, Instant};

// ---------- F1: hand-built attackers who know the code ----------

const PSK_SALT: &[u8] = b"pctwin/v1/pairing-psk";
const PROLOGUE_LABEL: &[u8] = b"pctwin/v1/pairing-prologue";
const NOISE: &str = "Noise_NNpsk0_25519_ChaChaPoly_BLAKE2s";

fn psk_from(key: &[u8]) -> [u8; 32] {
    let mut psk = [0u8; 32];
    Hkdf::<Sha256>::new(Some(PSK_SALT), key)
        .expand(b"noise psk", &mut psk)
        .unwrap();
    psk
}

fn prologue(id: &[u8], spake_a: &[u8], spake_b: &[u8]) -> Vec<u8> {
    let mut p = PROLOGUE_LABEL.to_vec();
    p.push(1);
    p.extend(id);
    p.extend(spake_a);
    p.extend(spake_b);
    p
}

fn match_number_of(handshake_hash: &[u8]) -> u8 {
    let mut okm = [0u8; 8];
    Hkdf::<Sha256>::new(Some(b"pctwin/v1/match-number"), handshake_hash)
        .expand(b"match number", &mut okm)
        .unwrap();
    10 + (u64::from_le_bytes(okm) % 90) as u8
}

/// An attacker who knows the code reimplements the RECEIVER from the public protocol and also
/// computes the match number from its own handshake, exactly as the Skeptic's bypass did.
/// Returns the sender's choosing state and the number the attacker computed.
fn raw_receiver(sender: &mut RotatingSender, now: Instant) -> (SenderChoosing, u8) {
    let code = sender.code().unwrap();
    let m1 = sender.message_1().unwrap().to_vec();
    let id = m1[2..10].to_vec();
    let spake_a = m1[10..43].to_vec();
    let (st, spake_b) = Spake2::<Ed25519Group>::start_b(
        &Password::new(code.as_bytes()),
        &Identity::new(b"pctwin/v1/sender"),
        &Identity::new(b"pctwin/v1/receiver"),
    );
    let psk = psk_from(&st.finish(&spake_a).unwrap());
    let pro = prologue(&id, &spake_a, &spake_b);
    let mut hs = snow::Builder::new(NOISE.parse().unwrap())
        .psk(0, &psk)
        .unwrap()
        .prologue(&pro)
        .unwrap()
        .build_initiator()
        .unwrap();
    let mut buf = [0u8; 128];
    let n = hs.write_message(&[], &mut buf).unwrap();
    let mut m2 = vec![1u8, 2];
    m2.extend(&id);
    m2.extend(&spake_b);
    m2.extend(&buf[..n]);
    let (choosing, m3) = sender.receive(&m2, now).unwrap();
    hs.read_message(&m3[2..], &mut buf).unwrap();
    let computed = match_number_of(hs.get_handshake_hash());
    (choosing, computed)
}

/// An attacker who knows the code reimplements the SENDER, so it can seal any approval it likes.
/// Returns the real receiver's waiting state and the attacker's transport.
fn raw_sender(code: &str, now: Instant) -> (ReceiverAwaitingApproval, snow::TransportState) {
    let id = [7u8; 8];
    let (st, spake_a) = Spake2::<Ed25519Group>::start_a(
        &Password::new(code.as_bytes()),
        &Identity::new(b"pctwin/v1/sender"),
        &Identity::new(b"pctwin/v1/receiver"),
    );
    let mut m1 = vec![1u8, 1];
    m1.extend(id);
    m1.extend(&spake_a);
    let (receiver, m2) = ReceiverSession::respond(&PairingCode::parse(code).unwrap(), &m1).unwrap();
    let spake_b = &m2[10..43];
    let noise_1 = &m2[43..];
    let psk = psk_from(&st.finish(spake_b).unwrap());
    let pro = prologue(&id, &spake_a, spake_b);
    let mut hs = snow::Builder::new(NOISE.parse().unwrap())
        .psk(0, &psk)
        .unwrap()
        .prologue(&pro)
        .unwrap()
        .build_responder()
        .unwrap();
    let mut buf = [0u8; 128];
    hs.read_message(noise_1, &mut buf).unwrap();
    let n = hs.write_message(&[], &mut buf).unwrap();
    let mut m3 = vec![1u8, 3];
    m3.extend(&buf[..n]);
    let waiting = receiver.receive(&m3, now).unwrap();
    (waiting, hs.into_transport_mode().unwrap())
}

fn sealed_approval(tr: &mut snow::TransportState, plain: &[u8]) -> Vec<u8> {
    let mut out = [0u8; 64];
    let k = tr.write_message(plain, &mut out).unwrap();
    let mut m4 = vec![1u8, 4];
    m4.extend(&out[..k]);
    m4
}

#[test]
fn control_the_hand_built_receiver_completes_the_handshake_and_knows_the_number() {
    // Proves the attack harness is faithful: it really derives the right number.
    let now = Instant::now();
    let mut sender = RotatingSender::new(now).unwrap();
    let (choosing, computed) = raw_receiver(&mut sender, now);
    assert!(choosing.choices().contains(&computed));
}

#[test]
fn knowing_the_number_does_not_unlock_the_sender_without_a_person_picking_on_it() {
    // Before the redesign, a code-holder who computed the number paired 100 times out of 100.
    // Now the sender has no way to accept anything from the other side; only `choose`, called
    // by the person at the sender, unlocks it. A person at their real new laptop sees a number
    // from a different session, which rarely appears among the three offered.
    let mut matched = 0;
    let trials = 900;
    for _ in 0..trials {
        let now = Instant::now();
        let mut sender = RotatingSender::new(now).unwrap();
        let (choosing, _attacker_knows) = raw_receiver(&mut sender, now);
        // The person's real new laptop paired with nobody in this session: its number is
        // independent of this session's choices.
        let real_receiver_number = {
            let code = PairingCode::generate().unwrap();
            let (s, m1) = SenderSession::start(&code, now);
            let (r, m2) = ReceiverSession::respond(&code.clone(), &m1).unwrap();
            let (_c, m3) = s.receive(&m2, now).unwrap();
            r.receive(&m3, now).unwrap().match_number()
        };
        if choosing.choices().contains(&real_receiver_number) {
            matched += 1;
        }
    }
    // The chance is 3 in 90 (about 3.3%); allow generous room for randomness.
    assert!(
        matched < trials / 10,
        "the real number appeared in {matched} of {trials} attacker sessions"
    );
}

#[test]
fn a_wrong_pick_on_the_sender_cancels_pairing() {
    let now = Instant::now();
    let mut sender = RotatingSender::new(now).unwrap();
    let (choosing, computed) = raw_receiver(&mut sender, now);
    let wrong = *choosing.choices().iter().find(|&&c| c != computed).unwrap();
    assert!(matches!(
        choosing.choose(wrong, now),
        Err(PairingError::WrongNumber)
    ));
}

#[test]
fn control_the_hand_built_sender_is_approved_with_the_right_label_and_number() {
    let now = Instant::now();
    let code = PairingCode::generate().unwrap().digits();
    let (waiting, mut tr) = raw_sender(&code, now);
    let mut plain = b"pctwin/v1/approved".to_vec();
    plain.push(waiting.match_number());
    let m4 = sealed_approval(&mut tr, &plain);
    assert!(waiting.receive_approval(&m4, now).is_ok());
}

#[test]
fn an_approval_with_the_wrong_number_is_rejected() {
    let now = Instant::now();
    let code = PairingCode::generate().unwrap().digits();
    let (waiting, mut tr) = raw_sender(&code, now);
    let wrong = MATCH_NUMBER_RANGE
        .clone()
        .find(|&n| n != waiting.match_number())
        .unwrap();
    let mut plain = b"pctwin/v1/approved".to_vec();
    plain.push(wrong);
    let m4 = sealed_approval(&mut tr, &plain);
    let rejected = waiting.receive_approval(&m4, now).unwrap_err();
    assert_eq!(rejected.error(), PairingError::WrongNumber);
    assert!(rejected.into_retry().is_none());
}

#[test]
fn an_approval_with_a_wrong_label_or_no_number_is_rejected() {
    for bad_label in [
        &b""[..],
        b"pctwin/v1/WRONG!!",
        b"pctwin/v1/approve",
        b"pctwin/v1/confirmed",
        b"PCTWIN/V1/APPROVED",
    ] {
        let now = Instant::now();
        let code = PairingCode::generate().unwrap().digits();
        let (waiting, mut tr) = raw_sender(&code, now);
        let mut plain = bad_label.to_vec();
        plain.push(waiting.match_number());
        let m4 = sealed_approval(&mut tr, &plain);
        let rejected = waiting.receive_approval(&m4, now).unwrap_err();
        assert!(
            rejected.into_retry().is_none(),
            "label {bad_label:?} was not checked"
        );
    }
    let now = Instant::now();
    let code = PairingCode::generate().unwrap().digits();
    let (waiting, mut tr) = raw_sender(&code, now);
    let m4 = sealed_approval(&mut tr, b"pctwin/v1/approved");
    assert!(waiting.receive_approval(&m4, now).is_err());
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

// ---------- F4: junk does not cancel; waiting times out ----------

fn up_to_approval(now: Instant) -> (ReceiverAwaitingApproval, Vec<u8>) {
    let code = PairingCode::generate().unwrap();
    let (s, m1) = SenderSession::start(&code, now);
    let (r, m2) = ReceiverSession::respond(&code.clone(), &m1).unwrap();
    let (choosing, m3) = s.receive(&m2, now).unwrap();
    let waiting = r.receive(&m3, now).unwrap();
    let (_p, m4) = choosing.choose(waiting.match_number(), now).unwrap();
    (waiting, m4)
}

#[test]
fn junk_approvals_do_not_cancel_the_wait() {
    let now = Instant::now();
    let (mut waiting, m4) = up_to_approval(now);
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
        let rejected = waiting.receive_approval(&j, now).unwrap_err();
        waiting = rejected
            .into_retry()
            .expect("junk must hand the waiting state back");
    }
    assert!(waiting.receive_approval(&m4, now).is_ok());
}

#[test]
fn an_approval_after_the_deadline_is_refused() {
    let now = Instant::now();
    let (waiting, m4) = up_to_approval(now);
    assert_eq!(waiting.deadline(), now + CONFIRMATION_TIMEOUT);
    let late = now + CONFIRMATION_TIMEOUT + Duration::from_millis(1);
    let rejected = waiting.receive_approval(&m4, late).unwrap_err();
    assert_eq!(rejected.error(), PairingError::Expired);
    assert!(rejected.into_retry().is_none());
}

#[test]
fn an_approval_arriving_exactly_at_the_deadline_is_accepted() {
    let now = Instant::now();
    let (waiting, m4) = up_to_approval(now);
    let deadline = waiting.deadline();
    assert!(waiting.receive_approval(&m4, deadline).is_ok());
}

#[test]
fn a_pick_on_the_sender_after_its_deadline_is_refused() {
    let now = Instant::now();
    let code = PairingCode::generate().unwrap();
    let (s, m1) = SenderSession::start(&code, now);
    let (r, m2) = ReceiverSession::respond(&code.clone(), &m1).unwrap();
    let (choosing, m3) = s.receive(&m2, now).unwrap();
    let number = r.receive(&m3, now).unwrap().match_number();
    assert_eq!(choosing.deadline(), now + CONFIRMATION_TIMEOUT);
    let late = choosing.deadline() + Duration::from_millis(1);
    assert!(matches!(
        choosing.choose(number, late),
        Err(PairingError::Expired)
    ));
}

#[test]
fn a_malformed_message_3_does_not_cancel_the_receiver() {
    let now = Instant::now();
    let code = PairingCode::generate().unwrap();
    let (s, m1) = SenderSession::start(&code, now);
    let (r, m2) = ReceiverSession::respond(&code.clone(), &m1).unwrap();
    let (_choosing, m3) = s.receive(&m2, now).unwrap();

    let mut r = r;
    for junk in [vec![], vec![1u8], vec![1, 4, 1, 2, 3], vec![7, 3, 0]] {
        r = r
            .receive(&junk, now)
            .unwrap_err()
            .into_retry()
            .expect("a malformed message 3 must hand the receiver back");
    }
    assert!(r.receive(&m3, now).is_ok());
}

#[test]
fn well_framed_junk_message_3_does_not_cancel_the_receiver() {
    let now = Instant::now();
    let code = PairingCode::generate().unwrap();
    let (s, m1) = SenderSession::start(&code, now);
    let (r, m2) = ReceiverSession::respond(&code.clone(), &m1).unwrap();
    let (_choosing, m3) = s.receive(&m2, now).unwrap();

    // Correct version and kind, then junk of every length up to a full-size message 3 and beyond.
    let mut r = r;
    let mut seed = 0x5eed_u64;
    for len in 0..=120usize {
        let mut junk = vec![1u8, 3];
        for _ in 0..len {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            junk.push((seed & 0xff) as u8);
        }
        r = r
            .receive(&junk, now)
            .unwrap_err()
            .into_retry()
            .unwrap_or_else(|| panic!("{len}-byte junk message 3 cancelled the receiver"));
    }
    // A genuine message 3 with one byte flipped is also junk to the receiver.
    for i in 2..m3.len() {
        let mut flipped = m3.clone();
        flipped[i] ^= 1;
        r = r
            .receive(&flipped, now)
            .unwrap_err()
            .into_retry()
            .unwrap_or_else(|| panic!("flipped byte {i} cancelled the receiver"));
    }
    assert!(
        r.receive(&m3, now).is_ok(),
        "the genuine message 3 still completes pairing"
    );
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
            matches!(
                ReceiverSession::respond(&code, &with_point(&m1, 11, p)),
                Err(PairingError::InvalidKey)
            ),
            "point {p:02x?} was not rejected by the point check"
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
        assert!(
            matches!(
                s.receive(&with_point(&m2, 11, p), now),
                Err(PairingError::InvalidKey)
            ),
            "point {p:02x?} was not rejected by the point check"
        );
    }
}
