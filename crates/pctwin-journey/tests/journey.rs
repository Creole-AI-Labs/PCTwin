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
    match u {
        OldUpdate::Choose(c) => Some(*c),
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
    let choices = next(&mut updates, choices_of).await;
    assert!(choices.contains(&shown), "{choices:?} {shown}");
    assert_eq!(choices.iter().collect::<BTreeSet<_>>().len(), 3);
    let (picked, approved) = tokio::join!(old.pick(shown), waiting.approval());
    let mut old_link = picked.unwrap();
    let mut new_link = approved.unwrap();
    next(&mut updates, |u| {
        matches!(u, OldUpdate::Paired).then_some(())
    })
    .await;
    new_link.send(b"hello").await.unwrap();
    assert_eq!(&old_link.recv().await.unwrap()[..], b"hello");
}

#[tokio::test]
async fn a_wrong_pick_ends_the_attempt_and_a_fresh_code_follows() {
    let (old, mut updates, found) = old_laptop().await;
    let (code, _) = next(&mut updates, code_of).await;
    let new = NewJourney::new(config());
    let waiting = new.connect(&found, &code).await.unwrap();
    let shown = waiting.number();
    let choices = next(&mut updates, choices_of).await;
    let wrong = *choices.iter().find(|&&n| n != shown).unwrap();
    let (picked, approved) = tokio::join!(old.pick(wrong), waiting.approval());
    assert_eq!(picked.err(), Some(Problem::WrongNumber));
    assert!(approved.is_err());
    next(&mut updates, |u| code_of(u).filter(|(c, _)| *c != code)).await;
}

#[tokio::test]
async fn numbers_that_do_not_match_can_be_declined_on_the_old_laptop() {
    let (old, mut updates, found) = old_laptop().await;
    let (code, _) = next(&mut updates, code_of).await;
    let new = NewJourney::new(config());
    let waiting = new.connect(&found, &code).await.unwrap();
    next(&mut updates, choices_of).await;
    old.decline().await;
    assert!(waiting.approval().await.is_err());
    next(&mut updates, |u| code_of(u).filter(|(c, _)| *c != code)).await;
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
        assert!(k.starts_with("pairing."), "{k}");
    }
}
