//! The whole pairing journey up to the number pick: the old laptop announces itself, the new laptop
//! finds it by its label, and connects to that laptop only.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use pctwin_discovery::{Found, Label};
use pctwin_link::{LinkConfig, LinkError};
use pctwin_pairing::{PairingCode, PairingError, RotatingSender};
use pctwin_session::{OldLaptop, connect_to, search};

const WAIT: Duration = Duration::from_secs(3);

fn config() -> LinkConfig {
    LinkConfig {
        connect_timeout: Duration::from_millis(500),
        ..LinkConfig::default()
    }
}

fn unique(name: &str) -> Label {
    let mut tag = [0u8; 4];
    getrandom::fill(&mut tag).unwrap();
    Label::named(&format!(
        "{name} {:02x}{:02x}{:02x}{:02x}",
        tag[0], tag[1], tag[2], tag[3]
    ))
    .unwrap()
}

fn sender() -> Arc<Mutex<RotatingSender>> {
    Arc::new(Mutex::new(RotatingSender::new(Instant::now()).unwrap()))
}

fn shown_code(sender: &Mutex<RotatingSender>) -> PairingCode {
    let mut s = sender.lock().unwrap();
    s.tick(Instant::now()).unwrap();
    PairingCode::parse(&s.code().unwrap()).unwrap()
}

fn mine<'a>(found: &'a [Found], label: &Label) -> Vec<&'a Found> {
    found.iter().filter(|f| &f.label == label).collect()
}

/// Waits at most 10 seconds, so a broken journey fails instead of hanging.
async fn within<T>(step: impl std::future::Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(10), step)
        .await
        .expect("the step finished within 10 seconds")
}

fn closed_port() -> SocketAddr {
    let spare = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = spare.local_addr().unwrap();
    drop(spare);
    addr
}

#[tokio::test]
async fn the_new_laptop_finds_the_old_laptop_by_its_label_and_pairs() {
    let label = unique("journey");
    let old = OldLaptop::start(label.clone(), config()).await.unwrap();
    assert_eq!(old.label(), &label);
    let sender = sender();
    let code = shown_code(&sender);

    // The new laptop searches and the person taps the label shown on the old laptop.
    let found = search(WAIT).await.unwrap();
    let chosen = mine(&found, &label);
    assert_eq!(chosen.len(), 1, "{found:?}");
    let chosen = chosen[0].clone();

    let guest = tokio::spawn(async move { connect_to(&chosen, &code, config()).await });
    let pending_host = within(old.next_peer(&sender)).await.unwrap();
    let pending_guest = guest.await.unwrap().unwrap();
    let shown = pending_guest.match_number();
    let mut host_link = pending_host.choose(shown, Instant::now()).await.unwrap();
    let mut guest_link = pending_guest.approval().await.unwrap();
    host_link.send(b"found you").await.unwrap();
    assert_eq!(guest_link.recv().await.unwrap().as_slice(), b"found you");
}

#[tokio::test]
async fn shuffle_or_a_new_name_changes_what_the_new_laptop_sees() {
    let first = unique("before");
    let mut old = OldLaptop::start(first.clone(), config()).await.unwrap();
    let second = unique("after");
    old.relabel(second.clone()).unwrap();
    assert_eq!(old.label(), &second);
    let found = search(WAIT).await.unwrap();
    assert!(mine(&found, &first).is_empty(), "the old label is gone");
    assert_eq!(mine(&found, &second).len(), 1, "{found:?}");
}

#[tokio::test]
async fn when_pairing_ends_the_old_laptop_disappears_and_stops_listening() {
    let label = unique("ending");
    let old = OldLaptop::start(label.clone(), config()).await.unwrap();
    let found = search(WAIT).await.unwrap();
    let chosen = mine(&found, &label)[0].clone();
    drop(old);
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(mine(&search(WAIT).await.unwrap(), &label).is_empty());
    let code = PairingCode::parse("123456").unwrap();
    let err = connect_to(&chosen, &code, config()).await.unwrap_err();
    assert!(matches!(err, LinkError::Unreachable), "got {err:?}");
}

#[tokio::test]
async fn an_unreachable_address_is_skipped_for_the_next_one() {
    let label = unique("skip");
    let old = OldLaptop::start(label.clone(), config()).await.unwrap();
    let sender = sender();
    let code = shown_code(&sender);
    let found = search(WAIT).await.unwrap();
    let mut chosen = mine(&found, &label)[0].clone();
    chosen.addrs.insert(0, closed_port()); // the first address does not answer

    let guest = tokio::spawn(async move { connect_to(&chosen, &code, config()).await });
    let pending_host = within(old.next_peer(&sender)).await.unwrap();
    let pending_guest = guest.await.unwrap().unwrap();
    assert!(
        pending_host
            .choices()
            .contains(&pending_guest.match_number())
    );
}

#[tokio::test]
async fn a_wrong_code_is_never_tried_at_a_second_address() {
    // Two old laptops; the found entry wrongly lists the second one's address after the first.
    let first = OldLaptop::start(unique("first"), config()).await.unwrap();
    let second = OldLaptop::start(unique("second"), config()).await.unwrap();
    let first_sender = sender();
    let second_sender = sender();
    let found = search(WAIT).await.unwrap();
    let mut chosen = mine(&found, first.label())[0].clone();
    let other = mine(&found, second.label())[0].addrs[0];
    chosen.addrs.truncate(1);
    chosen.addrs.push(other);

    let right = shown_code(&first_sender).digits();
    let last = right.as_bytes()[5] as char;
    let lead = if right.starts_with('9') { '0' } else { '9' };
    let wrong = PairingCode::parse(&format!("{lead}0000{last}")).unwrap();

    let serving_first = tokio::spawn({
        let s = first_sender.clone();
        async move { first.next_peer(&s).await.map(|p| p.choices()) }
    });
    let serving_second = tokio::spawn({
        let s = second_sender.clone();
        async move { second.next_peer(&s).await.map(|p| p.choices()) }
    });
    let err = connect_to(&chosen, &wrong, config()).await.unwrap_err();
    assert!(
        matches!(err, LinkError::Pairing(PairingError::HandshakeFailed)),
        "got {err:?}"
    );
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(first_sender.lock().unwrap().failed_attempts(), 1);
    assert_eq!(
        second_sender.lock().unwrap().failed_attempts(),
        0,
        "the second laptop was never tried"
    );
    serving_first.abort();
    serving_second.abort();
}

#[tokio::test]
async fn an_old_laptop_with_no_reachable_address_is_reported_unreachable() {
    let found = Found {
        label: unique("nowhere"),
        addrs: vec![closed_port(), closed_port()],
        same_label_nearby: false,
    };
    let code = PairingCode::parse("123456").unwrap();
    let err = connect_to(&found, &code, config()).await.unwrap_err();
    assert!(matches!(err, LinkError::Unreachable), "got {err:?}");
}

/// A listener that counts connections, standing in for a second address.
async fn counting_listener() -> (SocketAddr, Arc<std::sync::atomic::AtomicUsize>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let seen = count.clone();
    tokio::spawn(async move {
        while let Ok((_s, _)) = listener.accept().await {
            seen.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    });
    (addr, count)
}

#[tokio::test]
async fn once_an_address_answers_no_other_address_is_ever_tried() {
    let code = PairingCode::parse("123456").unwrap();
    // The first address accepts and hangs up, or accepts and says nothing.
    for hang_up in [true, false] {
        let first = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let first_addr = first.local_addr().unwrap();
        tokio::spawn(async move {
            while let Ok((s, _)) = first.accept().await {
                if hang_up {
                    drop(s);
                } else {
                    tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        drop(s);
                    });
                }
            }
        });
        let (second_addr, contacted) = counting_listener().await;
        let chosen = Found {
            label: unique("answers"),
            addrs: vec![first_addr, second_addr],
            same_label_nearby: false,
        };
        let quick = LinkConfig {
            step_timeout: Duration::from_millis(500),
            ..config()
        };
        let err = within(connect_to(&chosen, &code, quick)).await.unwrap_err();
        assert!(!matches!(err, LinkError::Unreachable), "got {err:?}");
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert_eq!(
            contacted.load(std::sync::atomic::Ordering::SeqCst),
            0,
            "hang_up={hang_up}: the second address must never be contacted"
        );
    }
}

#[tokio::test]
async fn pairing_works_after_a_shuffle() {
    let mut old = OldLaptop::start(unique("before shuffle"), config())
        .await
        .unwrap();
    let after = unique("after shuffle");
    old.relabel(after.clone()).unwrap();
    let sender = sender();
    let code = shown_code(&sender);
    let found = search(WAIT).await.unwrap();
    let chosen = mine(&found, &after)[0].clone();
    let guest = tokio::spawn(async move { connect_to(&chosen, &code, config()).await });
    let pending_host = within(old.next_peer(&sender)).await.unwrap();
    let pending_guest = guest.await.unwrap().unwrap();
    assert!(
        pending_host
            .choices()
            .contains(&pending_guest.match_number())
    );
}
