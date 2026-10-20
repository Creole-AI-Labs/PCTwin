//! A code shown before the laptop slept never stays usable after it wakes. The sender's clock
//! (`Instant`) does not count sleep on every system, so a code with 40 seconds left when the lid
//! closed could look live for 40 more seconds after a night's sleep. The sender also reads the
//! calendar clock, which keeps counting through sleep: when the calendar clock has moved on more
//! than the sender's clock since the last look, the laptop slept, and both live codes are retired
//! at once.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use pctwin_pairing::{
    CODE_LIFETIME, PairingCode, PairingError, ReceiverSession, RotatingSender, SenderStatus,
    WAKE_GAP,
};

/// A calendar clock the test moves by hand.
#[derive(Clone)]
struct Wall(Arc<Mutex<SystemTime>>);

impl Wall {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_800_000_000),
        )))
    }
    fn forward(&self, by: Duration) {
        *self.0.lock().unwrap() += by;
    }
    fn back(&self, by: Duration) {
        *self.0.lock().unwrap() -= by;
    }
    fn sender(&self, now: Instant) -> RotatingSender {
        let wall = self.clone();
        RotatingSender::with_wall_clock(now, move || *wall.0.lock().unwrap()).unwrap()
    }
}

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

fn wrong_for(code: &str) -> &'static str {
    if code == "000000" { "000002" } else { "000000" }
}

#[test]
fn the_wake_gap_is_a_few_seconds() {
    assert_eq!(WAKE_GAP, Duration::from_secs(5));
}

#[test]
fn after_sleep_a_fresh_code_shows_at_once_with_a_full_minute() {
    let wall = Wall::new();
    let start = Instant::now();
    let mut sender = wall.sender(start);
    let before = sender.message_1().unwrap().to_vec();

    // The lid closes 20 seconds in, for an hour; the sender's clock saw only a moment.
    let t = start + Duration::from_secs(20);
    wall.forward(Duration::from_secs(20));
    assert!(!sender.tick(t).unwrap(), "no sleep yet");
    wall.forward(Duration::from_secs(3600));
    let woke = t + Duration::from_millis(200);
    assert!(sender.tick(woke).unwrap(), "waking shows a new code");
    assert_ne!(sender.message_1().unwrap(), before.as_slice());
    assert_eq!(
        sender.status(),
        SenderStatus::Showing {
            code_expires_at: woke + CODE_LIFETIME
        },
        "the new code gets a full minute from waking"
    );
}

#[test]
fn a_code_shown_before_sleep_is_refused_after_waking_without_counting_as_a_wrong_try() {
    let wall = Wall::new();
    let start = Instant::now();
    let mut sender = wall.sender(start);
    let code = sender.code().unwrap();
    let msg1 = sender.message_1().unwrap().to_vec();

    wall.forward(Duration::from_secs(600));
    // The reply arrives first after waking, before the screen's own tick.
    assert!(matches!(
        attempt(&mut sender, &code, &msg1, start + Duration::from_secs(1)),
        Err(PairingError::UnknownSession)
    ));
    assert_eq!(sender.failed_attempts(), 0);
}

#[test]
fn the_code_replaced_just_before_sleep_is_retired_too() {
    let wall = Wall::new();
    let start = Instant::now();
    let mut sender = wall.sender(start);
    let old_code = sender.code().unwrap();
    let old_msg1 = sender.message_1().unwrap().to_vec();

    // The minute is up: a new code, and the old one in its grace period.
    let rotated = start + CODE_LIFETIME;
    wall.forward(CODE_LIFETIME);
    assert!(sender.tick(rotated).unwrap());
    let shown = sender.code().unwrap();
    let shown_msg1 = sender.message_1().unwrap().to_vec();

    wall.forward(Duration::from_secs(120));
    let woke = rotated + Duration::from_secs(1);
    assert!(sender.tick(woke).unwrap());
    for (code, msg1) in [(&old_code, &old_msg1), (&shown, &shown_msg1)] {
        assert!(matches!(
            attempt(&mut sender, code, msg1, woke),
            Err(PairingError::UnknownSession)
        ));
    }
    assert_eq!(sender.failed_attempts(), 0);
}

#[test]
fn the_new_code_after_waking_pairs() {
    let wall = Wall::new();
    let start = Instant::now();
    let mut sender = wall.sender(start);
    wall.forward(Duration::from_secs(90));
    let woke = start + Duration::from_secs(1);
    sender.tick(woke).unwrap();
    let code = sender.code().unwrap();
    let msg1 = sender.message_1().unwrap().to_vec();
    assert!(attempt(&mut sender, &code, &msg1, woke).is_ok());
}

#[test]
fn time_passing_normally_changes_nothing() {
    let wall = Wall::new();
    let start = Instant::now();
    let mut sender = wall.sender(start);
    let before = sender.message_1().unwrap().to_vec();
    for step in 1..=50u32 {
        wall.forward(Duration::from_millis(200));
        assert!(
            !sender
                .tick(start + Duration::from_millis(200) * step)
                .unwrap()
        );
    }
    assert_eq!(sender.message_1().unwrap(), before.as_slice());
}

#[test]
fn a_calendar_jump_under_the_wake_gap_changes_nothing() {
    let wall = Wall::new();
    let start = Instant::now();
    let mut sender = wall.sender(start);
    let before = sender.message_1().unwrap().to_vec();
    wall.forward(WAKE_GAP);
    assert!(!sender.tick(start).unwrap());
    assert_eq!(sender.message_1().unwrap(), before.as_slice());
}

#[test]
fn the_calendar_clock_set_back_changes_nothing_and_later_sleep_is_still_seen() {
    let wall = Wall::new();
    let start = Instant::now();
    let mut sender = wall.sender(start);
    let before = sender.message_1().unwrap().to_vec();
    wall.back(Duration::from_secs(3600));
    assert!(!sender.tick(start + Duration::from_secs(1)).unwrap());
    assert_eq!(sender.message_1().unwrap(), before.as_slice());

    wall.forward(Duration::from_secs(300));
    assert!(sender.tick(start + Duration::from_secs(2)).unwrap());
    assert_ne!(sender.message_1().unwrap(), before.as_slice());
}

#[test]
fn the_senders_own_clock_running_ahead_is_not_sleep() {
    // Systems whose clock does count sleep: the minute simply runs out, as when awake.
    let wall = Wall::new();
    let start = Instant::now();
    let mut sender = wall.sender(start);
    wall.forward(Duration::from_secs(30));
    assert!(!sender.tick(start + Duration::from_secs(30)).unwrap());
    wall.forward(Duration::from_secs(31));
    assert!(sender.tick(start + Duration::from_secs(61)).unwrap());
}

#[test]
fn waking_never_unlocks() {
    let wall = Wall::new();
    let start = Instant::now();
    let mut sender = wall.sender(start);
    let mut t = start;
    while sender.status() != SenderStatus::Locked {
        sender.tick(t).unwrap();
        let code = sender.code().unwrap();
        let msg1 = sender.message_1().unwrap().to_vec();
        let _ = attempt(&mut sender, wrong_for(&code), &msg1, t);
        t += Duration::from_secs(20);
    }
    wall.forward(Duration::from_secs(3600));
    assert!(!sender.tick(t).unwrap());
    assert_eq!(sender.status(), SenderStatus::Locked);
    assert!(sender.code().is_none());
}

#[test]
fn waking_during_a_pause_keeps_the_pause_and_a_fresh_code_follows() {
    let wall = Wall::new();
    let start = Instant::now();
    let mut sender = wall.sender(start);
    let code = sender.code().unwrap();
    let msg1 = sender.message_1().unwrap().to_vec();
    let _ = attempt(&mut sender, wrong_for(&code), &msg1, start);
    let until = start + Duration::from_secs(1);
    assert_eq!(sender.status(), SenderStatus::CoolingDown { until });

    wall.forward(Duration::from_secs(600));
    assert!(!sender.tick(start).unwrap());
    assert_eq!(sender.status(), SenderStatus::CoolingDown { until });
    assert_eq!(sender.failed_attempts(), 1, "waking forgives nothing");

    assert!(sender.tick(until).unwrap());
    let fresh = sender.code().unwrap();
    let fresh_msg1 = sender.message_1().unwrap().to_vec();
    assert!(attempt(&mut sender, &fresh, &fresh_msg1, until).is_ok());
}
