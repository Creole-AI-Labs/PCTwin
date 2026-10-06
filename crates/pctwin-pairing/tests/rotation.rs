//! The sender's code changes every minute, after any attempt, and keeps a short grace period
//! so someone who was mid-typing when it changed can still finish.

use pctwin_pairing::{
    CODE_LIFETIME, PairingCode, PairingError, ROTATION_GRACE, ReceiverSession, RotatingSender,
    SenderStatus,
};
use std::time::{Duration, Instant};

/// A receiver types `code` after receiving `msg1`, and the sender handles the reply.
fn attempt(
    sender: &mut RotatingSender,
    code: &str,
    msg1: &[u8],
    now: Instant,
) -> Result<(), PairingError> {
    let typed = PairingCode::parse(code).unwrap();
    let (_receiver, msg2) = ReceiverSession::respond(&typed, msg1, Instant::now())?;
    sender.receive(&msg2, now).map(|_| ())
}

#[test]
fn the_code_lasts_one_minute_with_fifteen_seconds_grace() {
    assert_eq!(CODE_LIFETIME, Duration::from_secs(60));
    assert_eq!(ROTATION_GRACE, Duration::from_secs(15));
}

#[test]
fn a_new_code_appears_every_minute() {
    let start = Instant::now();
    let mut sender = RotatingSender::new(start).unwrap();
    let first = sender.message_1().unwrap().to_vec();
    assert_eq!(sender.expires_at(), start + CODE_LIFETIME);

    assert!(
        !sender
            .tick(start + CODE_LIFETIME - Duration::from_millis(1))
            .unwrap()
    );
    assert_eq!(
        sender.message_1().unwrap(),
        first.as_slice(),
        "no change before the minute is up"
    );

    assert!(sender.tick(start + CODE_LIFETIME).unwrap());
    assert_ne!(
        sender.message_1().unwrap(),
        first.as_slice(),
        "a new session after one minute"
    );
    assert_eq!(sender.expires_at(), start + CODE_LIFETIME * 2);
}

#[test]
fn pairing_works_with_the_current_code() {
    let now = Instant::now();
    let mut sender = RotatingSender::new(now).unwrap();
    let code = sender.code().unwrap();
    let msg1 = sender.message_1().unwrap().to_vec();
    assert!(attempt(&mut sender, &code, &msg1, now).is_ok());
}

#[test]
fn the_previous_code_still_works_during_the_grace_period() {
    let start = Instant::now();
    let mut sender = RotatingSender::new(start).unwrap();
    let old_code = sender.code().unwrap();
    let old_msg1 = sender.message_1().unwrap().to_vec();

    sender.tick(start + CODE_LIFETIME).unwrap();
    let late_but_in_grace = start + CODE_LIFETIME + ROTATION_GRACE - Duration::from_millis(1);
    sender.tick(late_but_in_grace).unwrap();
    assert!(attempt(&mut sender, &old_code, &old_msg1, late_but_in_grace).is_ok());
}

#[test]
fn the_previous_code_stops_working_after_the_grace_period() {
    let start = Instant::now();
    let mut sender = RotatingSender::new(start).unwrap();
    let old_code = sender.code().unwrap();
    let old_msg1 = sender.message_1().unwrap().to_vec();

    sender.tick(start + CODE_LIFETIME).unwrap();
    let too_late = start + CODE_LIFETIME + ROTATION_GRACE;
    sender.tick(too_late).unwrap();
    assert!(matches!(
        attempt(&mut sender, &old_code, &old_msg1, too_late),
        Err(PairingError::UnknownSession)
    ));
}

#[test]
fn a_wrong_attempt_pauses_then_shows_a_fresh_code() {
    let now = Instant::now();
    let mut sender = RotatingSender::new(now).unwrap();
    let code = sender.code().unwrap();
    let msg1 = sender.message_1().unwrap().to_vec();
    let wrong = if code == "000000" { "000001" } else { "000000" };

    assert!(attempt(&mut sender, wrong, &msg1, now).is_err());
    assert_eq!(sender.failed_attempts(), 1);
    assert_eq!(
        sender.status(),
        SenderStatus::CoolingDown {
            until: now + Duration::from_secs(1)
        }
    );
    assert!(
        sender.code().is_none() && sender.message_1().is_none(),
        "no code during the pause"
    );

    let after = now + Duration::from_secs(1);
    assert!(sender.tick(after).unwrap());
    let fresh = sender.message_1().unwrap().to_vec();
    assert_ne!(fresh, msg1, "a fresh session after the pause");
    assert_eq!(
        sender.expires_at(),
        after + CODE_LIFETIME,
        "the new code gets a full minute"
    );

    // The burned code cannot be tried again, even with the right digits.
    assert!(matches!(
        attempt(&mut sender, &code, &msg1, after),
        Err(PairingError::UnknownSession)
    ));
}

#[test]
fn each_code_gets_exactly_one_attempt_even_during_grace() {
    let start = Instant::now();
    let mut sender = RotatingSender::new(start).unwrap();
    let old_code = sender.code().unwrap();
    let old_msg1 = sender.message_1().unwrap().to_vec();
    sender.tick(start + CODE_LIFETIME).unwrap();

    let t = start + CODE_LIFETIME + Duration::from_secs(1);
    let wrong = if old_code == "000000" {
        "000001"
    } else {
        "000000"
    };
    assert!(attempt(&mut sender, wrong, &old_msg1, t).is_err());

    // After the pause, the old code is gone even though its grace period has not ended.
    let after = t + Duration::from_secs(1);
    sender.tick(after).unwrap();
    assert!(matches!(
        attempt(&mut sender, &old_code, &old_msg1, after),
        Err(PairingError::UnknownSession)
    ));
}

#[test]
fn a_reply_for_an_unknown_session_does_not_burn_the_current_code() {
    let now = Instant::now();
    let mut sender = RotatingSender::new(now).unwrap();
    let code = sender.code().unwrap();
    let msg1 = sender.message_1().unwrap().to_vec();

    // A reply built against some other sender's session.
    let mut stranger = RotatingSender::new(now).unwrap();
    let stranger_code = stranger.code().unwrap();
    let stranger_msg1 = stranger.message_1().unwrap().to_vec();
    let (_r, foreign_msg2) = ReceiverSession::respond(
        &PairingCode::parse(&stranger_code).unwrap(),
        &stranger_msg1,
        Instant::now(),
    )
    .unwrap();
    assert!(matches!(
        sender.receive(&foreign_msg2, now),
        Err(PairingError::UnknownSession)
    ));

    // The real code still works afterwards.
    assert_eq!(sender.message_1().unwrap(), msg1.as_slice());
    assert!(attempt(&mut sender, &code, &msg1, now).is_ok());
    let _ = &mut stranger;
}

#[test]
fn garbage_replies_do_not_burn_the_current_code() {
    let now = Instant::now();
    let mut sender = RotatingSender::new(now).unwrap();
    let code = sender.code().unwrap();
    let msg1 = sender.message_1().unwrap().to_vec();
    for junk in [
        &b""[..],
        b"\x01",
        b"\x01\x02",
        b"\x01\x02short",
        &[0xff; 200],
    ] {
        assert!(sender.receive(junk, now).is_err());
    }
    assert!(attempt(&mut sender, &code, &msg1, now).is_ok());
}

#[test]
fn the_shown_code_is_the_one_that_works() {
    let now = Instant::now();
    let mut sender = RotatingSender::new(now).unwrap();
    for _ in 0..5 {
        sender.tick(now + CODE_LIFETIME * 10).unwrap_or(false);
    }
    let code = sender.code().unwrap();
    assert_eq!(code.len(), 6);
    let msg1 = sender.message_1().unwrap().to_vec();
    assert!(attempt(&mut sender, &code, &msg1, now + CODE_LIFETIME * 10).is_ok());
}

// ---------- even/odd codes, so the new laptop can say which live code it holds ----------

fn parity_of(code: &str) -> u8 {
    PairingCode::parse(code).unwrap().parity()
}

#[test]
fn parity_is_the_last_digit_even_or_odd() {
    assert_eq!(parity_of("123456"), 0);
    assert_eq!(parity_of("000001"), 1);
    assert_eq!(parity_of("999999"), 1);
}

#[test]
fn codes_alternate_even_and_odd_through_rotations_failures_and_grace() {
    let start = Instant::now();
    let mut sender = RotatingSender::new(start).unwrap();
    let mut t = start;
    let mut last = parity_of(&sender.code().unwrap());
    for round in 0..40u32 {
        if round % 3 == 0 {
            // A failed attempt replaces the code; wait out the pause.
            let code = sender.code().unwrap();
            let wrong = if code == "000000" { "000002" } else { "000000" };
            let msg1 = sender.message_1().unwrap().to_vec();
            let _ = attempt(&mut sender, wrong, &msg1, t);
            t += Duration::from_secs(10);
            sender.tick(t).unwrap();
            if sender.status() == SenderStatus::Locked {
                sender.unlock(t).unwrap();
            }
        } else {
            t += CODE_LIFETIME;
            sender.tick(t).unwrap();
        }
        let now_parity = parity_of(&sender.code().unwrap());
        if round % 3 != 0 {
            assert_ne!(
                now_parity, last,
                "round {round}: a rotation must flip even/odd"
            );
        }
        last = now_parity;
    }
}

#[test]
fn a_code_replaced_while_the_person_typed_it_still_pairs_during_the_grace() {
    let start = Instant::now();
    let mut sender = RotatingSender::new(start).unwrap();
    let typed = sender.code().unwrap();
    let t = start + CODE_LIFETIME + Duration::from_secs(5);
    sender.tick(t).unwrap();
    assert_ne!(sender.code().unwrap(), typed, "a new code is showing");

    // The new laptop says which code it holds (even or odd) and gets that code's message 1.
    let msg1 = sender.message_1_for(parity_of(&typed)).unwrap().to_vec();
    assert!(attempt(&mut sender, &typed, &msg1, t).is_ok());
}

#[test]
fn after_the_grace_only_the_code_shown_now_can_be_asked_for() {
    let start = Instant::now();
    let mut sender = RotatingSender::new(start).unwrap();
    let typed = sender.code().unwrap();
    let t = start + CODE_LIFETIME + ROTATION_GRACE + Duration::from_millis(1);
    sender.tick(t).unwrap();
    assert!(sender.message_1_for(parity_of(&typed)).is_none());
    let shown = sender.code().unwrap();
    assert_eq!(sender.message_1_for(parity_of(&shown)), sender.message_1());
}

#[test]
fn nothing_is_handed_out_while_paused_or_locked() {
    let start = Instant::now();
    let mut sender = RotatingSender::new(start).unwrap();
    let code = sender.code().unwrap();
    let wrong = if code == "000000" { "000002" } else { "000000" };
    let msg1 = sender.message_1().unwrap().to_vec();
    let _ = attempt(&mut sender, wrong, &msg1, start);
    assert!(sender.message_1_for(0).is_none() && sender.message_1_for(1).is_none());
}

#[test]
fn codes_stay_random_apart_from_the_last_digit() {
    let start = Instant::now();
    let mut sender = RotatingSender::new(start).unwrap();
    let mut seen = [[false; 10]; 6];
    let mut t = start;
    for _ in 0..3000 {
        for (i, c) in sender.code().unwrap().chars().enumerate() {
            seen[i][c.to_digit(10).unwrap() as usize] = true;
        }
        t += CODE_LIFETIME;
        sender.tick(t).unwrap();
    }
    assert!(
        seen.iter().all(|pos| pos.iter().all(|&s| s)),
        "every digit should appear in every position"
    );
}

#[test]
fn after_a_pause_the_spent_code_is_reported_expired_not_charged_again() {
    // With no code in its grace period, the code shown after a pause has the other even/odd
    // than the code a wrong guess just spent, so the spent code's holder is told it expired.
    let start = Instant::now();
    let mut sender = RotatingSender::new(start).unwrap();
    let spent = sender.code().unwrap();
    let wrong = if spent == "000000" {
        "000002"
    } else {
        "000000"
    };
    let msg1 = sender.message_1().unwrap().to_vec();
    let _ = attempt(&mut sender, wrong, &msg1, start);
    sender.tick(start + Duration::from_secs(2)).unwrap();
    assert!(sender.message_1_for(parity_of(&spent)).is_none());
}
