//! Extra lanes (Security Design A, "Extra lanes"): more connections between two laptops that
//! already paired, with no new code. Each lane runs its own Noise `NNpsk0` handshake with fresh
//! ephemeral keys; its pre-shared key is derived from the paired session and the lane number.
//!
//! ```text
//! new laptop -> old laptop : lane message 1 (lane number + Noise handshake 1)
//! old laptop -> new laptop : lane message 2 (Noise handshake 2)
//! new laptop -> old laptop : lane message 3 (sealed confirmation of the lane number)
//! ```
//!
//! The new laptop opens lanes (it connects to the old laptop, as in pairing); the old laptop
//! accepts a lane only after message 3 proves the opener is live and holds the session's keys.

use pctwin_pairing::{
    LaneAccepting, MAX_LANE_OPENINGS, PROTOCOL_VERSION, Paired, PairingCode, PairingError,
    ReceiverSession, SenderSession, Transport,
};
use std::time::{Duration, Instant};

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

/// Opens one lane: (old laptop's end, new laptop's end).
fn lane(old: &mut Paired, new: &mut Paired) -> (Transport, Transport) {
    let now = Instant::now();
    let (opening, l1) = new.open_lane(now).unwrap();
    let (accepting, l2) = old.accept_lane(&l1, now).unwrap();
    let (new_end, l3) = opening.receive(&l2, now).unwrap();
    let old_end = accepting.confirm(&l3, now).unwrap();
    (old_end, new_end)
}

#[test]
fn a_lane_opens_without_a_new_code_and_carries_data_both_ways() {
    let (mut old, mut new) = pair();
    let (mut old_end, mut new_end) = lane(&mut old, &mut new);
    let sealed = old_end.seal(b"block 7").unwrap();
    assert_eq!(&new_end.open(&sealed).unwrap()[..], b"block 7");
    let sealed = new_end.seal(b"receipt 7").unwrap();
    assert_eq!(&old_end.open(&sealed).unwrap()[..], b"receipt 7");
    // The first connection still works alongside it.
    let sealed = old.transport_mut().seal(b"main").unwrap();
    assert_eq!(&new.transport_mut().open(&sealed).unwrap()[..], b"main");
}

#[test]
fn every_lane_has_its_own_keys() {
    let (mut old, mut new) = pair();
    let (mut old_1, _new_1) = lane(&mut old, &mut new);
    let (_old_2, mut new_2) = lane(&mut old, &mut new);
    // Lane 1's message does not open on lane 2, nor on the first connection.
    let sealed = old_1.seal(b"secret").unwrap();
    assert_eq!(new_2.open(&sealed).unwrap_err(), PairingError::Transport);
    assert_eq!(
        new.transport_mut().open(&sealed).unwrap_err(),
        PairingError::Transport
    );
}

#[test]
fn lane_numbers_count_up_and_are_never_reused() {
    let (mut old, mut new) = pair();
    let now = Instant::now();
    let (a, _) = new.open_lane(now).unwrap();
    assert_eq!(a.lane(), 1);
    // Lane 1 is abandoned (the connection failed), and the next one is lane 2, not lane 1 again.
    drop(a);
    let (b, l1) = new.open_lane(now).unwrap();
    assert_eq!(b.lane(), 2);
    let (accepting, _) = old.accept_lane(&l1, now).unwrap();
    assert_eq!(accepting.lane(), 2);
}

#[test]
fn a_replayed_lane_opening_is_refused() {
    let (mut old, mut new) = pair();
    let now = Instant::now();
    let (opening, l1) = new.open_lane(now).unwrap();
    let (accepting, l2) = old.accept_lane(&l1, now).unwrap();
    assert_eq!(accepting.lane(), 1);
    // Someone who copied lane message 1 off the network sends it again.
    assert_eq!(
        old.accept_lane(&l1, now).unwrap_err(),
        PairingError::LaneRefused
    );
    // The genuine lane still completes.
    let (_new_end, l3) = opening.receive(&l2, now).unwrap();
    accepting.confirm(&l3, now).unwrap();
}

#[test]
fn a_stranger_cannot_open_a_lane_or_use_up_its_number() {
    let (mut old, mut new) = pair();
    let (_other_old, mut other_new) = pair();
    let now = Instant::now();
    // A lane opening from another pairing has the wrong keys.
    let (_, stranger) = other_new.open_lane(now).unwrap();
    assert!(old.accept_lane(&stranger, now).is_err());
    // Junk of every length is refused too, and none of it uses up lane 1.
    for len in [0, 1, 2, 6, 40, 80, 600] {
        let junk = vec![0x41; len];
        assert!(old.accept_lane(&junk, now).is_err());
    }
    let (opening, l1) = new.open_lane(now).unwrap();
    let (accepting, l2) = old.accept_lane(&l1, now).unwrap();
    assert_eq!(accepting.lane(), 1);
    let (_e, l3) = opening.receive(&l2, now).unwrap();
    accepting.confirm(&l3, now).unwrap();
}

#[test]
fn a_changed_lane_number_is_refused() {
    let (mut old, mut new) = pair();
    let now = Instant::now();
    let (_opening, mut l1) = new.open_lane(now).unwrap();
    // The lane number travels in the clear after the version and kind; it is also bound into the
    // handshake, so changing it breaks the handshake.
    assert_eq!(l1[0], PROTOCOL_VERSION);
    assert_eq!(&l1[2..6], &1u32.to_be_bytes());
    l1[5] = 9;
    assert!(old.accept_lane(&l1, now).is_err());
    l1[5] = 1;
    assert!(old.accept_lane(&l1, now).is_ok());
}

#[test]
fn the_old_laptop_accepts_a_lane_only_after_the_confirmation() {
    let (mut old, mut new) = pair();
    let (mut other_old, mut other_new) = pair();
    let now = Instant::now();
    let (opening, l1) = new.open_lane(now).unwrap();
    let (accepting, l2) = old.accept_lane(&l1, now).unwrap();
    let (_e, _l3) = opening.receive(&l2, now).unwrap();
    // A confirmation from another session does not count.
    let (o2, l1b) = other_new.open_lane(now).unwrap();
    let (_a2, l2b) = other_old.accept_lane(&l1b, now).unwrap();
    let (_e2, l3b) = o2.receive(&l2b, now).unwrap();
    assert!(accepting.confirm(&l3b, now).is_err());
}

#[test]
fn a_tampered_confirmation_is_refused() {
    let (mut old, mut new) = pair();
    let now = Instant::now();
    let (opening, l1) = new.open_lane(now).unwrap();
    let (accepting, l2) = old.accept_lane(&l1, now).unwrap();
    let (_e, l3) = opening.receive(&l2, now).unwrap();
    let mut wrong = l3.clone();
    let last = wrong.len() - 1;
    wrong[last] ^= 1;
    assert!(accepting.confirm(&wrong, now).is_err());
}

#[test]
fn only_the_new_laptop_opens_lanes_and_only_the_old_laptop_accepts_them() {
    let (mut old, mut new) = pair();
    let now = Instant::now();
    assert_eq!(
        old.open_lane(now).map(|_| ()).unwrap_err(),
        PairingError::LaneRefused
    );
    let (_o, l1) = new.open_lane(now).unwrap();
    assert_eq!(
        new.accept_lane(&l1, now).map(|_| ()).unwrap_err(),
        PairingError::LaneRefused
    );
}

#[test]
fn a_reply_from_someone_without_the_keys_is_refused_by_the_opener() {
    let (_old, mut new) = pair();
    let (mut other_old, mut other_new) = pair();
    let now = Instant::now();
    let (opening, _l1) = new.open_lane(now).unwrap();
    let (_o, l1b) = other_new.open_lane(now).unwrap();
    let (_a, l2b) = other_old.accept_lane(&l1b, now).unwrap();
    assert!(opening.receive(&l2b, now).is_err());
}

#[test]
fn lanes_that_take_too_long_end() {
    let (mut old, mut new) = pair();
    let now = Instant::now();
    let late = now + Duration::from_secs(31);
    let (opening, l1) = new.open_lane(now).unwrap();
    let (accepting, l2) = old.accept_lane(&l1, now).unwrap();
    assert_eq!(
        opening.receive(&l2, late).map(|_| ()).unwrap_err(),
        PairingError::Expired
    );
    let (opening, l1) = new.open_lane(now).unwrap();
    let (accepting2, l2) = old.accept_lane(&l1, now).unwrap();
    let (_e, l3) = opening.receive(&l2, now).unwrap();
    assert_eq!(
        accepting2.confirm(&l3, late).map(|_| ()).unwrap_err(),
        PairingError::Expired
    );
    let _: LaneAccepting = accepting;
}

#[test]
fn a_session_opens_a_limited_number_of_lanes() {
    let (_old, mut new) = pair();
    let now = Instant::now();
    for _ in 0..MAX_LANE_OPENINGS {
        new.open_lane(now).unwrap();
    }
    assert_eq!(
        new.open_lane(now).map(|_| ()).unwrap_err(),
        PairingError::LaneRefused
    );
}

#[test]
fn the_old_laptop_refuses_lane_numbers_outside_the_limit() {
    let (mut old, mut new) = pair();
    let now = Instant::now();
    let (_o, mut l1) = new.open_lane(now).unwrap();
    for n in [0u32, MAX_LANE_OPENINGS + 1, u32::MAX] {
        l1[2..6].copy_from_slice(&n.to_be_bytes());
        assert_eq!(
            old.accept_lane(&l1, now).map(|_| ()).unwrap_err(),
            PairingError::LaneRefused
        );
    }
}
