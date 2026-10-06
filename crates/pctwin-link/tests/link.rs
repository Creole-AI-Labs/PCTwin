//! Acceptance tests for pairing over a real network connection (Security Design Part A).
//!
//! The old laptop (host) listens; the new laptop (guest) connects after the person typed the code
//! and says whether that code ends in an even or odd digit. Only one guest can be pairing at a
//! time; everyone else is told busy, paused, expired or locked in plain terms.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use pctwin_link::{Host, LinkConfig, LinkError, connect, is_local_peer};
use pctwin_pairing::{CODE_LIFETIME, PairingCode, PairingError, RotatingSender};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const KIND_PAIRING: u8 = 1;
const KIND_BUSY: u8 = 2;
const KIND_HELLO: u8 = 4;

fn fast() -> LinkConfig {
    LinkConfig {
        step_timeout: Duration::from_millis(400),
        first_step_timeout: Duration::from_millis(400),
        silence_penalty: Duration::ZERO,
    }
}

async fn host_with(config: LinkConfig) -> (Host, Arc<Mutex<RotatingSender>>) {
    let host = Host::bind("127.0.0.1:0".parse().unwrap(), config)
        .await
        .unwrap();
    let sender = Arc::new(Mutex::new(RotatingSender::new(Instant::now()).unwrap()));
    (host, sender)
}

async fn host_and_sender() -> (Host, Arc<Mutex<RotatingSender>>) {
    host_with(fast()).await
}

fn shown_code(sender: &Mutex<RotatingSender>) -> PairingCode {
    PairingCode::parse(&sender.lock().unwrap().code().unwrap()).unwrap()
}

fn wrong_code_for(sender: &Mutex<RotatingSender>) -> PairingCode {
    // Same even/odd as the code shown, so the host serves it and it counts as a real guess.
    let right = shown_code(sender);
    let digits = right.digits();
    let last = digits.as_bytes()[5];
    let first = if digits.as_bytes()[0] == b'9' {
        '0'
    } else {
        '9'
    };
    PairingCode::parse(&format!("{first}0000{}", last as char)).unwrap()
}

async fn read_frame(raw: &mut TcpStream) -> (u8, Vec<u8>) {
    let mut header = [0u8; 3];
    raw.read_exact(&mut header).await.unwrap();
    let mut body = vec![0u8; u16::from_be_bytes([header[1], header[2]]) as usize];
    raw.read_exact(&mut body).await.unwrap();
    (header[0], body)
}

fn spawn_host(host: Host, sender: Arc<Mutex<RotatingSender>>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let _ = host.next_peer(&sender).await;
    })
}

// ---------- the happy path ----------

#[tokio::test]
async fn two_laptops_pair_over_a_real_connection_and_exchange_sealed_data() {
    let (host, sender) = host_and_sender().await;
    let addr = host.local_addr().unwrap();
    let code = shown_code(&sender);

    let guest = tokio::spawn(async move { connect(addr, &code, fast()).await.unwrap() });
    let pending_host = host.next_peer(&sender).await.unwrap();
    let pending_guest = guest.await.unwrap();

    // The person reads the new laptop's number and picks it on the old laptop.
    let shown = pending_guest.match_number();
    assert!(pending_host.choices().contains(&shown));
    let mut host_link = pending_host.choose(shown, Instant::now()).await.unwrap();
    let mut guest_link = pending_guest.approval().await.unwrap();

    host_link.send(b"hello from the old laptop").await.unwrap();
    assert_eq!(
        guest_link.recv().await.unwrap().as_slice(),
        b"hello from the old laptop"
    );
    guest_link.send(b"and back").await.unwrap();
    assert_eq!(host_link.recv().await.unwrap().as_slice(), b"and back");
}

#[tokio::test]
async fn a_code_that_changed_while_the_person_typed_it_still_pairs() {
    let (host, sender) = host_and_sender().await;
    let addr = host.local_addr().unwrap();
    let typed = shown_code(&sender);
    // The minute is up just as the person finishes typing: a new code is showing.
    sender
        .lock()
        .unwrap()
        .tick(Instant::now() + CODE_LIFETIME + Duration::from_secs(5))
        .unwrap();
    assert_ne!(shown_code(&sender).digits(), typed.digits());

    let guest = tokio::spawn(async move { connect(addr, &typed, fast()).await });
    let pending_host = host.next_peer(&sender).await.unwrap();
    let pending_guest = guest.await.unwrap().unwrap();
    assert!(
        pending_host
            .choices()
            .contains(&pending_guest.match_number())
    );
    assert_eq!(sender.lock().unwrap().failed_attempts(), 0);
}

// ---------- one device at a time ----------

#[tokio::test]
async fn while_one_device_is_pairing_any_other_is_told_busy_and_cannot_touch_it() {
    let (host, sender) = host_and_sender().await;
    let addr = host.local_addr().unwrap();
    let parity = shown_code(&sender).parity();
    let serving = spawn_host(host, sender);

    // A first device holds the slot: it said hello and got message 1 ("typing").
    let mut first = TcpStream::connect(addr).await.unwrap();
    first.write_all(&[KIND_HELLO, 0, 1, parity]).await.unwrap();
    assert_eq!(read_frame(&mut first).await.0, KIND_PAIRING);

    // A second device tries to slip a reply in; it is told busy and closed without being read.
    let mut raw = TcpStream::connect(addr).await.unwrap();
    let _ = raw.write_all(&[1, 0, 4, 1, 2, 3, 4]).await;
    let mut reply = Vec::new();
    let _ = raw.read_to_end(&mut reply).await;
    assert_eq!(
        reply,
        vec![KIND_BUSY, 0, 0],
        "the second device is told busy"
    );

    // The app's own connect reports it in plain terms.
    let code = PairingCode::parse("123456").unwrap();
    let err = connect(addr, &code, fast()).await.unwrap_err();
    assert!(matches!(err, LinkError::Busy), "got {err:?}");
    serving.abort();
}

#[tokio::test]
async fn a_silent_device_is_dropped_and_the_next_one_pairs_with_the_same_code() {
    let (host, sender) = host_and_sender().await;
    let addr = host.local_addr().unwrap();
    let code = shown_code(&sender);

    let silent = tokio::spawn(async move {
        let mut raw = TcpStream::connect(addr).await.unwrap();
        let mut sink = Vec::new();
        let _ = raw.read_to_end(&mut sink).await; // says nothing at all
    });
    let real = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(700)).await; // after the silent one times out
        connect(addr, &code, fast()).await
    });
    let pending_host = host.next_peer(&sender).await.unwrap();
    let pending_guest = real.await.unwrap().unwrap();
    silent.await.unwrap();
    assert!(
        pending_host
            .choices()
            .contains(&pending_guest.match_number())
    );
    assert_eq!(
        sender.lock().unwrap().failed_attempts(),
        0,
        "silence is not a guess"
    );
}

#[tokio::test]
async fn an_address_that_held_the_slot_in_silence_is_refused_for_a_while() {
    let config = LinkConfig {
        silence_penalty: Duration::from_millis(1500),
        ..fast()
    };
    let (host, sender) = host_with(config).await;
    let addr = host.local_addr().unwrap();
    let code = shown_code(&sender);
    let serving = spawn_host(host, sender.clone());

    // A silent device holds the slot until the first-step wait runs out.
    let mut silent = TcpStream::connect(addr).await.unwrap();
    let mut sink = Vec::new();
    let _ = silent.read_to_end(&mut sink).await;

    // Coming straight back from the same address, it is refused without holding the slot.
    let started = Instant::now();
    let err = connect(addr, &code, config).await.unwrap_err();
    assert!(matches!(err, LinkError::Busy), "got {err:?}");
    assert!(
        started.elapsed() < Duration::from_millis(300),
        "refused at once"
    );

    // Once the penalty is over, a device at that address can pair.
    tokio::time::sleep(Duration::from_millis(1600)).await;
    assert!(connect(addr, &code, config).await.is_ok());
    serving.abort();
}

#[tokio::test]
async fn an_oversized_message_is_refused_and_the_next_device_pairs() {
    let (host, sender) = host_and_sender().await;
    let addr = host.local_addr().unwrap();
    let code = shown_code(&sender);
    let parity = code.parity();

    let flooder = tokio::spawn(async move {
        let mut raw = TcpStream::connect(addr).await.unwrap();
        raw.write_all(&[KIND_HELLO, 0, 1, parity]).await.unwrap();
        let _msg1 = read_frame(&mut raw).await;
        // Announce a 60 KB pairing message: far over the limit.
        let _ = raw.write_all(&[1, 0xea, 0x60]).await;
        let _ = raw.write_all(&vec![0u8; 60_000]).await;
    });
    let real = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        connect(addr, &code, fast()).await
    });
    let pending_host = host.next_peer(&sender).await.unwrap();
    let pending_guest = real.await.unwrap().unwrap();
    flooder.await.unwrap();
    assert!(
        pending_host
            .choices()
            .contains(&pending_guest.match_number())
    );
}

#[tokio::test]
async fn a_flood_of_connections_does_not_stop_the_real_device_pairing() {
    let (host, sender) = host_and_sender().await;
    let addr = host.local_addr().unwrap();
    let code = shown_code(&sender);

    let serving = tokio::spawn(async move {
        let pending = host.next_peer(&sender).await.unwrap();
        pending.choices()
    });
    // A burst of 300 connections at once that open and drop, as a flood would.
    let burst: Vec<_> = (0..300)
        .map(|_| {
            tokio::spawn(async move {
                tokio::time::timeout(Duration::from_secs(2), TcpStream::connect(addr))
                    .await
                    .is_ok_and(|r| r.is_ok())
            })
        })
        .collect();
    let mut connected = 0;
    for b in burst {
        connected += usize::from(b.await.unwrap());
    }
    assert!(connected > 0);
    // The listener survived; the real device pairs.
    let pending_guest = tokio::time::timeout(Duration::from_secs(10), connect(addr, &code, fast()))
        .await
        .expect("the real device is served within 10 seconds")
        .unwrap();
    let choices = serving.await.unwrap();
    assert!(choices.contains(&pending_guest.match_number()));
}

// ---------- plain answers instead of timeouts ----------

#[tokio::test]
async fn a_wrong_code_fails_for_the_guest_and_counts_as_one_guess() {
    let (host, sender) = host_and_sender().await;
    let addr = host.local_addr().unwrap();
    let wrong = wrong_code_for(&sender);
    let serving = spawn_host(host, sender.clone());
    assert!(connect(addr, &wrong, fast()).await.is_err());
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(sender.lock().unwrap().failed_attempts(), 1);
    serving.abort();
}

#[tokio::test]
async fn right_after_a_wrong_code_the_next_device_is_told_to_wait_a_moment() {
    let (host, sender) = host_and_sender().await;
    let addr = host.local_addr().unwrap();
    let wrong = wrong_code_for(&sender);
    let serving = spawn_host(host, sender.clone());
    assert!(connect(addr, &wrong, fast()).await.is_err());

    let started = Instant::now();
    let err = connect(addr, &wrong, fast()).await.unwrap_err();
    assert!(
        matches!(err, LinkError::Pairing(PairingError::CoolingDown)),
        "got {err:?}"
    );
    assert!(
        started.elapsed() < Duration::from_millis(300),
        "told at once"
    );
    serving.abort();
}

#[tokio::test]
async fn a_code_that_is_no_longer_live_is_reported_as_expired() {
    let (host, sender) = host_and_sender().await;
    let addr = host.local_addr().unwrap();
    // Only one code is live, so a code with the other even/odd cannot be current.
    let shown = shown_code(&sender).digits();
    let other_last = if shown.as_bytes()[5].is_multiple_of(2) {
        '1'
    } else {
        '0'
    };
    let stale = PairingCode::parse(&format!("{}{other_last}", &shown[..5])).unwrap();
    let serving = spawn_host(host, sender.clone());

    let err = connect(addr, &stale, fast()).await.unwrap_err();
    assert!(
        matches!(err, LinkError::Pairing(PairingError::Expired)),
        "got {err:?}"
    );
    assert_eq!(sender.lock().unwrap().failed_attempts(), 0, "not a guess");
    serving.abort();
}

#[tokio::test]
async fn a_locked_old_laptop_stops_serving_until_the_person_starts_again() {
    let (host, sender) = host_and_sender().await;
    {
        // Five failed guesses, spaced past each pause.
        let mut s = sender.lock().unwrap();
        let start = Instant::now();
        for i in 0..5u64 {
            let t = start + Duration::from_secs(100 * i);
            s.tick(t).unwrap();
            let mut junk = vec![1u8, 2];
            junk.extend(&s.message_1().unwrap()[2..10]);
            junk.extend([7u8; 60]);
            assert!(s.receive(&junk, t).is_err());
        }
    }
    let err = host.next_peer(&sender).await.unwrap_err();
    assert!(
        matches!(err, LinkError::Pairing(PairingError::Locked)),
        "got {err:?}"
    );
}

// ---------- only the local network ----------

#[test]
fn only_local_network_addresses_are_served() {
    for ok in [
        "127.0.0.1",
        "10.1.2.3",
        "172.20.10.2",
        "192.168.1.20",
        "169.254.7.7",
        "::1",
        "fe80::1",
        "fd12:3456::1",
        "::ffff:192.168.0.5",
    ] {
        assert!(is_local_peer(ok.parse().unwrap()), "{ok} should be served");
    }
    for bad in [
        "8.8.8.8",
        "100.64.0.1",
        "203.0.113.9",
        "2001:4860::8888",
        "::ffff:8.8.8.8",
        "0.0.0.0",
        "255.255.255.255",
    ] {
        assert!(
            !is_local_peer(bad.parse().unwrap()),
            "{bad} must be refused"
        );
    }
}
