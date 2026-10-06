//! Acceptance and attack tests for PCTwin pairing (Security Design Part A).
//!
//! Flow: the sender (old laptop) shows a code; the receiver (new laptop) types it.
//!
//! ```text
//! 1. sender   -> receiver : message 1 (SPAKE2 A)
//! 2. receiver -> sender   : message 2 (SPAKE2 B + Noise handshake 1)
//! 3. sender   -> receiver : message 3 (Noise handshake 2)
//! 4. the sender shows one number; the receiver shows three and the person picks the match
//! 5. receiver -> sender   : message 4 (sealed confirmation)
//! ```
//!
//! Only after the right pick do both sides get the encrypted transport.

use pctwin_pairing::{
    CODE_LIFETIME, MATCH_NUMBER_RANGE, MAX_MESSAGE_LEN, PROTOCOL_VERSION, Paired, PairingCode,
    PairingError, ROTATION_GRACE, ReceiverSession, SenderSession,
};
use std::time::{Duration, Instant};

fn pair_with(
    sender_code: &PairingCode,
    typed: &PairingCode,
) -> Result<(Paired, Paired), PairingError> {
    let now = Instant::now();
    let (sender, msg1) = SenderSession::start(sender_code, now);
    let (receiver, msg2) = ReceiverSession::respond(typed, &msg1)?;
    let (sender_waiting, msg3) = sender.receive(&msg2, now)?;
    let choosing = receiver.receive(&msg3)?;
    // The person reads the sender's number and picks it on the receiver.
    let shown = sender_waiting.match_number();
    let (receiver_paired, msg4) = choosing.choose(shown)?;
    let sender_paired = sender_waiting.receive_confirmation(&msg4)?;
    Ok((sender_paired, receiver_paired))
}

// ---------- the happy path ----------

#[test]
fn correct_code_and_correct_pick_pairs() {
    let code = PairingCode::generate().expect("random code");
    let typed = PairingCode::parse(&code.digits()).expect("parse");
    pair_with(&code, &typed).expect("pairing succeeds");
}

fn up_to_choice(
    code: &PairingCode,
) -> (
    pctwin_pairing::SenderAwaitingConfirmation,
    pctwin_pairing::ReceiverChoosing,
) {
    let now = Instant::now();
    let (sender, msg1) = SenderSession::start(code, now);
    let (receiver, msg2) = ReceiverSession::respond(&code.clone(), &msg1).unwrap();
    let (sender_waiting, msg3) = sender.receive(&msg2, now).unwrap();
    (sender_waiting, receiver.receive(&msg3).unwrap())
}

#[test]
fn receiver_offers_three_different_numbers_including_the_senders() {
    let code = PairingCode::generate().unwrap();
    for _ in 0..200 {
        let (s, r) = up_to_choice(&code);
        let choices = r.choices();
        assert!(choices.contains(&s.match_number()));
        assert!(choices[0] != choices[1] && choices[1] != choices[2] && choices[0] != choices[2]);
        assert!(choices.iter().all(|c| MATCH_NUMBER_RANGE.contains(c)));
        assert!(MATCH_NUMBER_RANGE.contains(&s.match_number()));
    }
}

#[test]
fn the_correct_number_appears_in_every_position() {
    let code = PairingCode::generate().unwrap();
    let mut positions = [false; 3];
    for _ in 0..200 {
        let (s, r) = up_to_choice(&code);
        let i = r
            .choices()
            .iter()
            .position(|&c| c == s.match_number())
            .unwrap();
        positions[i] = true;
    }
    assert_eq!(
        positions, [true; 3],
        "the right answer must not always sit in the same place"
    );
}

#[test]
fn picking_a_wrong_number_cancels_pairing() {
    let code = PairingCode::generate().unwrap();
    let (s, r) = up_to_choice(&code);
    let wrong = *r
        .choices()
        .iter()
        .find(|&&c| c != s.match_number())
        .unwrap();
    assert!(matches!(r.choose(wrong), Err(PairingError::WrongNumber)));
}

#[test]
fn picking_a_number_that_was_not_offered_cancels_pairing() {
    let code = PairingCode::generate().unwrap();
    let (_s, r) = up_to_choice(&code);
    let not_offered = MATCH_NUMBER_RANGE
        .clone()
        .find(|n| !r.choices().contains(n))
        .unwrap();
    assert!(r.choose(not_offered).is_err());
}

#[test]
fn sender_rejects_a_forged_or_tampered_confirmation() {
    let code = PairingCode::generate().unwrap();
    let (s, r) = up_to_choice(&code);
    let n = s.match_number();
    let (_rp, mut msg4) = r.choose(n).unwrap();
    let last = msg4.len() - 1;
    msg4[last] ^= 0x01;
    assert!(s.receive_confirmation(&msg4).is_err());

    let (s2, _r2) = up_to_choice(&code);
    assert!(s2.receive_confirmation(b"confirmed").is_err());
}

#[test]
fn a_confirmation_from_another_session_is_rejected() {
    let code = PairingCode::generate().unwrap();
    let (s1, r1) = up_to_choice(&code);
    let n1 = s1.match_number();
    let (_p, msg4_from_1) = r1.choose(n1).unwrap();
    let (s2, _r2) = up_to_choice(&code);
    assert!(s2.receive_confirmation(&msg4_from_1).is_err());
}

#[test]
fn transport_works_both_ways_after_pairing() {
    let code = PairingCode::generate().unwrap();
    let (mut a, mut b) = pair_with(&code, &code.clone()).unwrap();
    let sealed = a
        .transport_mut()
        .seal(b"hello from the old laptop")
        .unwrap();
    assert_eq!(
        b.transport_mut().open(&sealed).unwrap(),
        b"hello from the old laptop"
    );
    let back = b.transport_mut().seal(b"and back").unwrap();
    assert_eq!(a.transport_mut().open(&back).unwrap(), b"and back");
}

#[test]
fn sealed_data_is_not_readable_as_plaintext() {
    let code = PairingCode::generate().unwrap();
    let (mut a, _b) = pair_with(&code, &code.clone()).unwrap();
    let secret = b"my tax return 2026";
    let sealed = a.transport_mut().seal(secret).unwrap();
    assert!(!sealed.windows(secret.len()).any(|w| w == secret));
}

#[test]
fn every_pairing_uses_fresh_keys() {
    let code = PairingCode::generate().unwrap();
    let (mut a1, _) = pair_with(&code, &code.clone()).unwrap();
    let (mut a2, _) = pair_with(&code, &code.clone()).unwrap();
    let c1 = a1.transport_mut().seal(b"same message").unwrap();
    let c2 = a2.transport_mut().seal(b"same message").unwrap();
    assert_ne!(
        c1, c2,
        "same code, same message, different sessions must not repeat ciphertext"
    );
}

// ---------- the code ----------

#[test]
fn codes_are_six_digits_and_leading_zeros_are_allowed() {
    for _ in 0..2000 {
        let code = PairingCode::generate().unwrap();
        let d = code.digits();
        assert_eq!(d.len(), 6);
        assert!(d.chars().all(|c| c.is_ascii_digit()));
    }
    assert!(PairingCode::parse("000123").is_ok());
}

#[test]
fn codes_vary_across_all_digit_positions() {
    let mut seen = [[false; 10]; 6];
    for _ in 0..5000 {
        let code = PairingCode::generate().unwrap();
        for (i, c) in code.digits().chars().enumerate() {
            seen[i][c.to_digit(10).unwrap() as usize] = true;
        }
    }
    assert!(
        seen.iter().all(|pos| pos.iter().all(|&s| s)),
        "every digit should appear in every position"
    );
}

#[test]
fn typed_codes_ignore_spaces_and_reject_anything_else() {
    assert_eq!(PairingCode::parse(" 123 456 ").unwrap().digits(), "123456");
    for bad in ["12345", "1234567", "12345a", "", "１２３４５６", "12-456"] {
        assert!(
            matches!(PairingCode::parse(bad), Err(PairingError::InvalidCode)),
            "accepted {bad:?}"
        );
    }
}

#[test]
fn code_expires_after_its_lifetime_and_grace() {
    let code = PairingCode::generate().unwrap();
    let start = Instant::now();
    let (sender, msg1) = SenderSession::start(&code, start);
    let (_receiver, msg2) = ReceiverSession::respond(&code.clone(), &msg1).unwrap();
    let late = start + CODE_LIFETIME + ROTATION_GRACE + Duration::from_millis(1);
    assert!(matches!(
        sender.receive(&msg2, late),
        Err(PairingError::Expired)
    ));
}

#[test]
fn code_just_inside_its_lifetime_still_works() {
    let code = PairingCode::generate().unwrap();
    let start = Instant::now();
    let (sender, msg1) = SenderSession::start(&code, start);
    let (_receiver, msg2) = ReceiverSession::respond(&code.clone(), &msg1).unwrap();
    assert!(
        sender
            .receive(&msg2, start + CODE_LIFETIME + ROTATION_GRACE)
            .is_ok()
    );
}

// ---------- attacks ----------

#[test]
fn wrong_code_fails_and_reveals_nothing_specific() {
    let code = PairingCode::parse("482913").unwrap();
    let guess = PairingCode::parse("482914").unwrap();
    let err = pair_with(&code, &guess).unwrap_err();
    assert!(matches!(err, PairingError::HandshakeFailed));
}

#[test]
fn a_session_allows_exactly_one_attempt() {
    // `receive` consumes the sender session, so a second guess cannot be tried against it.
    // This is enforced by the type system; this test documents the flow after a failure.
    let code = PairingCode::parse("111111").unwrap();
    let now = Instant::now();
    let (sender, msg1) = SenderSession::start(&code, now);
    let (_r, bad) =
        ReceiverSession::respond(&PairingCode::parse("222222").unwrap(), &msg1).unwrap();
    assert!(sender.receive(&bad, now).is_err());
    // `sender` has been moved; a fresh code and session are required.
}

#[test]
fn reflected_first_message_is_rejected() {
    // An attacker bounces the sender's own message back to it.
    let code = PairingCode::generate().unwrap();
    let now = Instant::now();
    let (sender, msg1) = SenderSession::start(&code, now);
    let mut reflected = msg1.clone();
    reflected[1] = 2; // pretend it is a message 2
    assert!(sender.receive(&reflected, now).is_err());
}

#[test]
fn receiver_rejects_its_own_message_reflected() {
    let code = PairingCode::generate().unwrap();
    let (_sender, msg1) = SenderSession::start(&code, Instant::now());
    let (receiver, msg2) = ReceiverSession::respond(&code.clone(), &msg1).unwrap();
    assert!(receiver.receive(&msg2).is_err());
}

#[test]
fn replayed_messages_from_an_old_session_are_rejected() {
    let code = PairingCode::generate().unwrap();
    let now = Instant::now();
    // Old session, fully recorded by an eavesdropper.
    let (old_sender, old_msg1) = SenderSession::start(&code, now);
    let (old_receiver, old_msg2) = ReceiverSession::respond(&code.clone(), &old_msg1).unwrap();
    let (_, old_msg3) = old_sender.receive(&old_msg2, now).unwrap();
    old_receiver.receive(&old_msg3).unwrap();

    // New session with the same code: replaying old message 2 to the new sender fails.
    let (new_sender, _new_msg1) = SenderSession::start(&code, now);
    assert!(new_sender.receive(&old_msg2, now).is_err());

    // Replaying old message 3 to a new receiver fails.
    let (_s, new_msg1) = SenderSession::start(&code, now);
    let (new_receiver, _) = ReceiverSession::respond(&code.clone(), &new_msg1).unwrap();
    assert!(new_receiver.receive(&old_msg3).is_err());
}

#[test]
fn tampering_with_any_byte_of_message_2_or_3_is_detected() {
    let code = PairingCode::generate().unwrap();
    let now = Instant::now();
    let (_, msg1) = SenderSession::start(&code, now);
    let (_, msg2) = ReceiverSession::respond(&code.clone(), &msg1).unwrap();
    for i in 0..msg2.len() {
        let (sender, msg1b) = SenderSession::start(&code, now);
        let (_, mut fresh2) = ReceiverSession::respond(&code.clone(), &msg1b).unwrap();
        fresh2[i] ^= 0x01;
        assert!(
            sender.receive(&fresh2, now).is_err(),
            "flip at byte {i} of message 2 went unnoticed"
        );
    }
    let _ = msg2;

    let (sender, msg1) = SenderSession::start(&code, now);
    let (receiver, msg2) = ReceiverSession::respond(&code.clone(), &msg1).unwrap();
    let (_, msg3) = sender.receive(&msg2, now).unwrap();
    for i in 0..msg3.len() {
        let mut bad = msg3.clone();
        bad[i] ^= 0x01;
        // Each attempt needs a fresh receiver because receive consumes it.
        let (s2, m1) = SenderSession::start(&code, now);
        let (r2, m2) = ReceiverSession::respond(&code.clone(), &m1).unwrap();
        let (_, mut m3) = s2.receive(&m2, now).unwrap();
        m3[i] ^= 0x01;
        assert!(
            r2.receive(&m3).is_err(),
            "flip at byte {i} of message 3 went unnoticed"
        );
    }
    let _ = (receiver, msg3);
}

#[test]
fn tampered_transport_data_is_rejected() {
    let code = PairingCode::generate().unwrap();
    let (mut a, mut b) = pair_with(&code, &code.clone()).unwrap();
    let mut sealed = a.transport_mut().seal(b"block 1").unwrap();
    let last = sealed.len() - 1;
    sealed[last] ^= 0x01;
    assert!(b.transport_mut().open(&sealed).is_err());
}

#[test]
fn a_man_in_the_middle_without_the_code_cannot_pair_with_either_side() {
    // The attacker sits between the laptops and must guess the code for each leg.
    let real = PairingCode::parse("731264").unwrap();
    let attacker_guess = PairingCode::parse("000000").unwrap();
    let now = Instant::now();

    // Leg 1: attacker pretends to be the receiver to the real sender.
    let (sender, msg1) = SenderSession::start(&real, now);
    let (_att_r, att_msg2) = ReceiverSession::respond(&attacker_guess, &msg1).unwrap();
    assert!(sender.receive(&att_msg2, now).is_err());

    // Leg 2: attacker pretends to be the sender to the real receiver.
    let (att_s, att_msg1) = SenderSession::start(&attacker_guess, now);
    let (receiver, msg2) = ReceiverSession::respond(&real, &att_msg1).unwrap();
    assert!(
        att_s.receive(&msg2, now).is_err(),
        "attacker cannot complete the handshake"
    );
    let _ = receiver;
}

#[test]
fn different_sessions_give_varied_match_numbers() {
    let code = PairingCode::generate().unwrap();
    let mut numbers = std::collections::HashSet::new();
    for _ in 0..200 {
        let (s, _) = up_to_choice(&code);
        numbers.insert(s.match_number());
    }
    assert!(
        numbers.len() > 40,
        "match numbers should spread across the range"
    );
}

// ---------- malformed input ----------

#[test]
fn wrong_protocol_version_is_reported() {
    let code = PairingCode::generate().unwrap();
    let (_, mut msg1) = SenderSession::start(&code, Instant::now());
    msg1[0] = PROTOCOL_VERSION + 1;
    assert!(matches!(
        ReceiverSession::respond(&code.clone(), &msg1),
        Err(PairingError::UnsupportedVersion(v)) if v == PROTOCOL_VERSION + 1
    ));
}

#[test]
fn oversized_and_truncated_messages_are_rejected_without_panicking() {
    let code = PairingCode::generate().unwrap();
    let now = Instant::now();
    let huge = vec![PROTOCOL_VERSION; MAX_MESSAGE_LEN + 1];
    assert!(ReceiverSession::respond(&code, &huge).is_err());
    let (_, msg1) = SenderSession::start(&code, now);
    for len in 0..msg1.len() {
        assert!(
            ReceiverSession::respond(&code, &msg1[..len]).is_err(),
            "accepted {len}-byte prefix"
        );
    }
    let (sender, msg1) = SenderSession::start(&code, now);
    let (_, msg2) = ReceiverSession::respond(&code.clone(), &msg1).unwrap();
    assert!(sender.receive(&msg2[..msg2.len() - 1], now).is_err());
}

#[test]
fn random_garbage_never_panics() {
    let code = PairingCode::generate().unwrap();
    let mut seed = 0x2454_f2e7_f36a_u64;
    for len in 0..300 {
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                (seed & 0xff) as u8
            })
            .collect();
        let _ = ReceiverSession::respond(&code, &bytes);
        let (sender, _) = SenderSession::start(&code, Instant::now());
        let _ = sender.receive(&bytes, Instant::now());
    }
}
