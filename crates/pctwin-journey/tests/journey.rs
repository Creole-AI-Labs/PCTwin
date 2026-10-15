//! The pairing journey as the screens see it (Task List 1.7, C03 to C05): the old laptop shows
//! its label and a code with a countdown, a new laptop connects with the code the person typed
//! (in any digits), both screens show their number, and only the right pick on the old laptop
//! pairs them. Every way it can go wrong reaches the screens as a translation key, never as
//! engine text. Run over a real connection on this laptop, without network discovery.

use std::collections::BTreeSet;
use std::time::Duration;

use pctwin_discovery::{Found, Label};
use pctwin_journey::{NewJourney, OldJourney, OldUpdate, Problem, Serving};
use pctwin_link::{Host, HostPending, LinkConfig, LinkError};
use pctwin_pairing::RotatingSender;
use tokio::sync::mpsc::UnboundedReceiver;

fn config() -> LinkConfig {
    LinkConfig {
        connect_timeout: Duration::from_millis(500),
        ..LinkConfig::default()
    }
}

/// An old laptop serving on this laptop's loopback, with a label but no announcement.
struct Loopback {
    host: Host,
    label: std::sync::Mutex<Label>,
}

impl Serving for Loopback {
    async fn next_peer(
        &self,
        sender: &std::sync::Mutex<RotatingSender>,
    ) -> Result<HostPending, LinkError> {
        self.host.next_peer(sender).await
    }
    fn label(&self) -> Label {
        self.label.lock().unwrap().clone()
    }
    fn relabel(&self, label: Label) -> Result<(), Problem> {
        *self.label.lock().unwrap() = label;
        Ok(())
    }
}

async fn old_laptop() -> (OldJourney<Loopback>, UnboundedReceiver<OldUpdate>, Found) {
    let host = Host::bind("127.0.0.1:0".parse().unwrap(), config())
        .await
        .unwrap();
    let found = Found {
        label: Label::named("Blue Fox").unwrap(),
        addrs: vec![host.local_addr().unwrap()],
        same_label_nearby: false,
    };
    let serving = Loopback {
        host,
        label: std::sync::Mutex::new(Label::named("Blue Fox").unwrap()),
    };
    let (old, updates) = OldJourney::begin(serving).unwrap();
    (old, updates, found)
}

/// The next update matching `want`, within five seconds.
async fn next<T>(
    updates: &mut UnboundedReceiver<OldUpdate>,
    want: impl Fn(&OldUpdate) -> Option<T>,
) -> T {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let u = updates.recv().await.expect("updates keep coming");
            if let Some(t) = want(&u) {
                return t;
            }
        }
    })
    .await
    .expect("the update came within five seconds")
}

fn code_of(u: &OldUpdate) -> Option<(String, u64)> {
    match u {
        OldUpdate::Code { code, seconds_left } => Some((code.clone(), *seconds_left)),
        _ => None,
    }
}

fn choices_of(u: &OldUpdate) -> Option<[u8; 3]> {
    round_of(u).map(|(_, numbers)| numbers)
}

fn round_of(u: &OldUpdate) -> Option<(u64, [u8; 3])> {
    match u {
        OldUpdate::Choose { round, numbers } => Some((*round, *numbers)),
        _ => None,
    }
}

fn label_of(u: &OldUpdate) -> Option<Label> {
    match u {
        OldUpdate::Label(l) => Some(l.clone()),
        _ => None,
    }
}

/// Full-width digits, as a Japanese or Chinese keyboard types them.
fn full_width(code: &str) -> String {
    code.chars()
        .map(|c| char::from_u32(0xFF10 + c.to_digit(10).unwrap()).unwrap())
        .collect()
}

#[tokio::test]
async fn the_old_laptop_shows_its_label_and_a_code_with_a_countdown() {
    let (_old, mut updates, _) = old_laptop().await;
    let label = next(&mut updates, label_of).await;
    assert_eq!(label, Label::named("Blue Fox").unwrap());
    let (code, seconds) = next(&mut updates, code_of).await;
    assert_eq!(code.len(), 6);
    assert!(code.bytes().all(|b| b.is_ascii_digit()));
    assert!((55..=60).contains(&seconds), "{seconds}");
}

#[tokio::test]
async fn the_right_pick_on_the_old_laptop_pairs_both() {
    let (old, mut updates, found) = old_laptop().await;
    let (code, _) = next(&mut updates, code_of).await;
    let new = NewJourney::new(config());
    let waiting = new.connect(&found, &full_width(&code)).await.unwrap();
    let shown = waiting.number();
    let (round, choices) = next(&mut updates, round_of).await;
    assert!(choices.contains(&shown), "{choices:?} {shown}");
    assert_eq!(choices.iter().collect::<BTreeSet<_>>().len(), 3);
    let (picked, approved) = tokio::join!(old.pick(round, shown), waiting.approval());
    let mut old_link = picked.unwrap();
    let mut new_link = approved.unwrap();
    next(&mut updates, |u| {
        matches!(u, OldUpdate::Paired).then_some(())
    })
    .await;
    new_link.send(b"hello").await.unwrap();
    assert_eq!(&old_link.recv().await.unwrap()[..], b"hello");
    // Paired: no more codes are shown or served.
    tokio::time::sleep(Duration::from_millis(1500)).await;
    while let Ok(u) = updates.try_recv() {
        assert!(code_of(&u).is_none(), "a code shown after pairing: {u:?}");
    }
}

#[tokio::test]
async fn a_wrong_pick_ends_the_attempt_and_a_fresh_code_follows() {
    let (old, mut updates, found) = old_laptop().await;
    let (code, _) = next(&mut updates, code_of).await;
    let new = NewJourney::new(config());
    let waiting = new.connect(&found, &code).await.unwrap();
    let shown = waiting.number();
    let (round, choices) = next(&mut updates, round_of).await;
    let wrong = *choices.iter().find(|&&n| n != shown).unwrap();
    let (picked, approved) = tokio::join!(old.pick(round, wrong), waiting.approval());
    assert_eq!(picked.err(), Some(Problem::WrongNumber));
    // The new laptop is told the numbers did not match, not that the connection was lost.
    assert_eq!(approved.err(), Some(Problem::NoMatch));
    next(&mut updates, |u| code_of(u).filter(|(c, _)| *c != code)).await;
}

#[tokio::test]
async fn numbers_that_do_not_match_can_be_declined_on_the_old_laptop() {
    let (old, mut updates, found) = old_laptop().await;
    let (code, _) = next(&mut updates, code_of).await;
    let new = NewJourney::new(config());
    let waiting = new.connect(&found, &code).await.unwrap();
    let (round, _) = next(&mut updates, round_of).await;
    old.decline(round).await;
    assert_eq!(waiting.approval().await.err(), Some(Problem::NoMatch));
    // Serving carries on: another new laptop with the fresh code reaches the pick.
    let (fresh, _) = next(&mut updates, |u| code_of(u).filter(|(c, _)| *c != code)).await;
    let again = new.connect(&found, &fresh).await.unwrap();
    let choices = next(&mut updates, choices_of).await;
    assert!(choices.contains(&again.number()));
}

#[tokio::test]
async fn a_code_that_is_not_six_digits_is_refused_before_anything_is_sent() {
    let (_old, _updates, found) = old_laptop().await;
    let new = NewJourney::new(config());
    for typed in ["12345", "12345a", "", "1234567"] {
        assert_eq!(
            new.connect(&found, typed).await.err(),
            Some(Problem::InvalidCode),
            "{typed:?}"
        );
    }
}

#[tokio::test]
async fn a_wrong_code_is_told_and_the_old_laptop_waits_a_moment() {
    let (_old, mut updates, found) = old_laptop().await;
    let (code, _) = next(&mut updates, code_of).await;
    // Same even/odd last digit, different first digit: a real, wrong guess.
    let first = if code.starts_with('9') { '0' } else { '9' };
    let wrong = format!("{first}{}", &code[1..]);
    let new = NewJourney::new(config());
    assert_eq!(
        new.connect(&found, &wrong).await.err(),
        Some(Problem::WrongCode)
    );
    next(&mut updates, |u| {
        matches!(u, OldUpdate::Wait { .. }).then_some(())
    })
    .await;
}

#[tokio::test]
async fn shuffle_and_rename_change_the_label_shown() {
    let (old, mut updates, _) = old_laptop().await;
    next(&mut updates, label_of).await;
    old.shuffle().unwrap();
    let shuffled = next(&mut updates, label_of).await;
    assert_ne!(shuffled, Label::named("Blue Fox").unwrap());
    old.rename("Kitchen laptop").unwrap();
    let renamed = next(&mut updates, label_of).await;
    assert_eq!(renamed, Label::named("Kitchen laptop").unwrap());
}

#[test]
fn every_problem_has_its_own_translation_key() {
    let all = Problem::all();
    let keys: BTreeSet<&str> = all.iter().map(Problem::key).collect();
    assert_eq!(keys.len(), all.len());
    for k in keys {
        assert!(k.starts_with("pairing-"), "{k}");
        // A Fluent message id: letters, digits and dashes only.
        assert!(k.chars().all(|c| c.is_ascii_lowercase() || c == '-'), "{k}");
    }
}

/// From the fresh-context review: a "none of these match" with nobody waiting (a double tap, a
/// tap after the time ran out) must never let a second laptop in while the person is picking.
#[tokio::test]
async fn a_stray_decline_never_lets_another_laptop_in_during_the_pick() {
    let (old, mut updates, found) = old_laptop().await;
    old.decline(0).await;
    old.decline(7).await;
    let (code, _) = next(&mut updates, code_of).await;
    let new = NewJourney::new(config());
    let first = new.connect(&found, &code).await.unwrap();
    let (round, _) = next(&mut updates, round_of).await;
    // A second laptop trying the code still on screen is not served while the first waits.
    let second = tokio::time::timeout(Duration::from_secs(2), new.connect(&found, &code)).await;
    assert!(
        !matches!(second, Ok(Ok(_))),
        "a second laptop reached the pick"
    );
    let (picked, approved) = tokio::join!(old.pick(round, first.number()), first.approval());
    assert!(picked.is_ok() && approved.is_ok());
}

/// A pick answers the numbers it was shown for: one meant for an earlier laptop never applies to
/// the next one.
#[tokio::test]
async fn a_pick_for_earlier_numbers_never_applies_to_the_next_laptop() {
    let (old, mut updates, found) = old_laptop().await;
    let (code, _) = next(&mut updates, code_of).await;
    let new = NewJourney::new(config());
    let first = new.connect(&found, &code).await.unwrap();
    let (earlier, _) = next(&mut updates, round_of).await;
    old.decline(earlier).await;
    assert!(first.approval().await.is_err());
    let (fresh, _) = next(&mut updates, |u| code_of(u).filter(|(c, _)| *c != code)).await;
    let second = new.connect(&found, &fresh).await.unwrap();
    let (round, _) = next(&mut updates, round_of).await;
    assert_ne!(round, earlier);
    assert_eq!(
        old.pick(earlier, second.number()).await.err(),
        Some(Problem::TimedOut)
    );
    let (picked, approved) = tokio::join!(old.pick(round, second.number()), second.approval());
    assert!(picked.is_ok() && approved.is_ok());
}

/// Once paired, the old laptop stops serving and stops listening: no other laptop can reach it.
#[tokio::test]
async fn once_paired_no_other_laptop_can_reach_the_old_laptop() {
    let (old, mut updates, found) = old_laptop().await;
    let (code, _) = next(&mut updates, code_of).await;
    let new = NewJourney::new(config());
    let waiting = new.connect(&found, &code).await.unwrap();
    let (round, _) = next(&mut updates, round_of).await;
    let (picked, approved) = tokio::join!(old.pick(round, waiting.number()), waiting.approval());
    let _links = (picked.unwrap(), approved.unwrap());
    tokio::time::sleep(Duration::from_millis(300)).await;
    let later = tokio::time::timeout(Duration::from_secs(3), new.connect(&found, "123456")).await;
    assert!(
        matches!(later, Ok(Err(Problem::Unreachable))),
        "after pairing: {:?}",
        later.as_ref().map(|r| r.as_ref().err())
    );
}

/// Two picks at once (a double tap): exactly one pairs.
#[tokio::test]
async fn two_picks_at_once_pair_exactly_once() {
    let (old, mut updates, found) = old_laptop().await;
    let (code, _) = next(&mut updates, code_of).await;
    let new = NewJourney::new(config());
    let waiting = new.connect(&found, &code).await.unwrap();
    let n = waiting.number();
    let (round, _) = next(&mut updates, round_of).await;
    let (a, b, approved) = tokio::join!(old.pick(round, n), old.pick(round, n), waiting.approval());
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    assert!(approved.is_ok());
}
