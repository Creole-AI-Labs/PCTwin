//! Resume (Security Design A, "Resume"): after the connection drops, the two laptops reconnect
//! without a new code, with fresh traffic keys, and only for the move the person approved. The
//! pairing code is never used again.
//!
//! ```text
//! new laptop -> old laptop : resume message 1 (attempt number + Noise handshake 1)
//! old laptop -> new laptop : resume message 2 (Noise handshake 2)
//! new laptop -> old laptop : resume message 3 (sealed confirmation of the attempt number)
//! ```
//!
//! Resume keys come from the paired session's lane keys and the approved plan's fingerprint, with
//! their own salt and label: a resume for another plan or another pairing fails, and no lane
//! message is ever taken for a resume or the other way round. Each attempt number works once;
//! the keys stop working [`RESUME_LIFETIME`] after the plan was approved.

use pctwin_pairing::{
    LaneKeys, MAX_RESUMES, Paired, PairingCode, PairingError, RESUME_LIFETIME, ReceiverSession,
    ResumeKeys, SenderSession, Transport,
};
use std::time::{Duration, Instant};

const PLAN: [u8; 32] = [0x11; 32];
const OTHER_PLAN: [u8; 32] = [0x22; 32];

/// Pairs two laptops: (old laptop, new laptop).
fn pair() -> (Paired, Paired) {
    let code = PairingCode::generate().unwrap();
    let now = Instant::now();
    let (sender, msg1) = SenderSession::start(&code, now);
    let (receiver, msg2) = ReceiverSession::respond(&code, &msg1, now).unwrap();
    let (sender_waiting, msg3) = sender.receive(&msg2, now).unwrap();
    let (waiting, msg4) = receiver.receive(&msg3, now).unwrap();
    let choosing = sender_waiting.receive_reveal(&msg4, now).unwrap();
    let (old, msg5) = choosing.choose(waiting.match_number(), now).unwrap();
    let new = waiting.receive_approval(&msg5, now).unwrap();
    (old, new)
}

/// Pairs, then both laptops make resume keys for `old_plan` and `new_plan` at `at`:
/// (old laptop's keys, new laptop's keys, old main link, new main link).
fn approved(
    old_plan: &[u8; 32],
    new_plan: &[u8; 32],
    at: Instant,
) -> (ResumeKeys, ResumeKeys, Transport, Transport) {
    let (old, new) = pair();
    let (old_t, old_lanes) = old.into_parts();
    let (new_t, new_lanes) = new.into_parts();
    (
        old_lanes.resume_keys(old_plan, at).unwrap(),
        new_lanes.resume_keys(new_plan, at).unwrap(),
        old_t,
        new_t,
    )
}

/// Runs one resume at `now`: (old laptop's new link, new laptop's new link).
fn resume(
    old: &mut ResumeKeys,
    new: &mut ResumeKeys,
    now: Instant,
) -> Result<(Transport, Transport), PairingError> {
    let (opening, r1) = new.open(now)?;
    let (accepting, r2) = old.accept(&r1, now)?;
    let (new_end, r3) = opening.receive(&r2, now)?;
    let old_end = accepting.confirm(&r3, now)?;
    Ok((old_end, new_end))
}

fn exchange(a: &mut Transport, b: &mut Transport) {
    let sealed = a.seal(b"block 7").unwrap();
    assert_eq!(&b.open(&sealed).unwrap()[..], b"block 7");
    let sealed = b.seal(b"receipt 7").unwrap();
    assert_eq!(&a.open(&sealed).unwrap()[..], b"receipt 7");
}

#[test]
fn after_a_drop_the_laptops_reconnect_without_a_code() {
    let now = Instant::now();
    let (mut old, mut new, _, _) = approved(&PLAN, &PLAN, now);
    let (mut a, mut b) = resume(&mut old, &mut new, now).unwrap();
    exchange(&mut a, &mut b);
    // And again after a second drop, with keys of its own.
    let (mut c, mut d) = resume(&mut old, &mut new, now).unwrap();
    exchange(&mut c, &mut d);
}

#[test]
fn a_resumed_link_has_fresh_keys() {
    let now = Instant::now();
    let (mut old, mut new, _old_main, mut new_main) = approved(&PLAN, &PLAN, now);
    let (mut old_end, _new_end) = resume(&mut old, &mut new, now).unwrap();
    // Something sealed on the old connection cannot be read on the resumed one.
    let sealed = new_main.seal(b"from before the drop").unwrap();
    assert!(old_end.open(&sealed).is_err());
}

#[test]
fn a_resume_for_another_plan_fails() {
    let now = Instant::now();
    let (mut old, mut new, _, _) = approved(&PLAN, &OTHER_PLAN, now);
    assert!(resume(&mut old, &mut new, now).is_err());
}

#[test]
fn a_resume_from_another_pairing_fails() {
    let now = Instant::now();
    let (mut old, _, _, _) = approved(&PLAN, &PLAN, now);
    let (_, mut stranger, _, _) = approved(&PLAN, &PLAN, now);
    assert!(resume(&mut old, &mut stranger, now).is_err());
}

#[test]
fn a_copied_resume_message_works_only_once() {
    let now = Instant::now();
    let (mut old, mut new, _, _) = approved(&PLAN, &PLAN, now);
    let (_opening, r1) = new.open(now).unwrap();
    assert!(old.accept(&r1, now).is_ok());
    assert_eq!(
        old.accept(&r1, now).map(|_| ()).unwrap_err(),
        PairingError::LaneRefused
    );
}

#[test]
fn junk_never_spends_an_attempt() {
    let now = Instant::now();
    let (mut old, mut new, _, _) = approved(&PLAN, &PLAN, now);
    let (_, r1) = new.open(now).unwrap();
    let mut forged = r1.clone();
    let last = forged.len() - 1;
    forged[last] ^= 1;
    assert!(old.accept(&forged, now).is_err());
    // The genuine message for the same attempt still works.
    assert!(old.accept(&r1, now).is_ok());
}

#[test]
fn a_lane_is_never_taken_for_a_resume_or_the_other_way_round() {
    let now = Instant::now();
    let (old, new) = pair();
    let (_, mut old_lanes) = old.into_parts();
    let (_, mut new_lanes) = new.into_parts();
    let mut old_resume = clone_resume(&old_lanes, now);
    let mut new_resume = clone_resume(&new_lanes, now);

    let (_, lane_1) = new_lanes.open_lane(now).unwrap();
    assert!(old_resume.accept(&lane_1, now).is_err());
    let (_, resume_1) = new_resume.open(now).unwrap();
    assert!(old_lanes.accept_lane(&resume_1, now).is_err());
}

fn clone_resume(lanes: &LaneKeys, now: Instant) -> ResumeKeys {
    lanes.resume_keys(&PLAN, now).unwrap()
}

#[test]
fn only_the_new_laptop_resumes_and_only_the_old_laptop_accepts() {
    let now = Instant::now();
    let (mut old, mut new, _, _) = approved(&PLAN, &PLAN, now);
    assert_eq!(
        old.open(now).map(|_| ()).unwrap_err(),
        PairingError::LaneRefused
    );
    let (_, r1) = new.open(now).unwrap();
    assert_eq!(
        new.accept(&r1, now).map(|_| ()).unwrap_err(),
        PairingError::LaneRefused
    );
}

#[test]
fn resume_keys_stop_working_after_their_lifetime() {
    let at = Instant::now();
    let (mut old, mut new, _, _) = approved(&PLAN, &PLAN, at);
    let late = at + RESUME_LIFETIME + Duration::from_secs(1);
    assert_eq!(
        new.open(late).map(|_| ()).unwrap_err(),
        PairingError::Expired
    );
    let (_, r1) = new.open(at).unwrap();
    assert_eq!(
        old.accept(&r1, late).map(|_| ()).unwrap_err(),
        PairingError::Expired
    );
    // Just inside the lifetime still works.
    let edge = at + RESUME_LIFETIME;
    assert!(resume(&mut old, &mut new, edge).is_ok());
}

#[test]
fn resumes_are_limited() {
    let now = Instant::now();
    let (_, mut new, _, _) = approved(&PLAN, &PLAN, now);
    for _ in 0..MAX_RESUMES {
        new.open(now).unwrap();
    }
    assert_eq!(
        new.open(now).map(|_| ()).unwrap_err(),
        PairingError::LaneRefused
    );
}
