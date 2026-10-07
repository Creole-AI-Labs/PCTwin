//! Acceptance tests for pairing over a real network connection (Security Design Part A).
//!
//! The old laptop (host) listens; the new laptop (guest) connects after the person typed the code
//! and says whether that code ends in an even or odd digit. Only one guest can be pairing at a
//! time; everyone else is told busy, paused, expired or locked in plain terms.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use pctwin_link::{Host, LinkConfig, LinkError, connect, is_local_peer};
use pctwin_pairing::{
    CODE_LIFETIME, PairingCode, PairingError, ROTATION_GRACE, ReceiverSession, RotatingSender,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const KIND_PAIRING: u8 = 1;
const KIND_BUSY: u8 = 2;
const KIND_HELLO: u8 = 4;
const KIND_PAUSED: u8 = 5;
const KIND_EXPIRED: u8 = 6;

fn fast() -> LinkConfig {
    LinkConfig {
        step_timeout: Duration::from_millis(400),
        handshake_timeout: Duration::from_millis(400),
        silence_penalty: Duration::ZERO,
        connect_timeout: Duration::from_millis(400),
    }
}

#[tokio::test]
async fn a_closed_address_is_reported_unreachable_without_sending_anything() {
    // Find a port nothing listens on.
    let spare = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = spare.local_addr().unwrap();
    drop(spare);
    let code = PairingCode::parse("123456").unwrap();
    let started = Instant::now();
    let err = connect(addr, &code, fast()).await.unwrap_err();
    assert!(matches!(err, LinkError::Unreachable), "got {err:?}");
    assert!(started.elapsed() < Duration::from_secs(2));
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
    // As the app does: bring the screen up to date, then read the code shown.
    let mut s = sender.lock().unwrap();
    s.tick(Instant::now()).unwrap();
    PairingCode::parse(&s.code().unwrap()).unwrap()
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

#[tokio::test]
async fn a_code_that_runs_out_mid_handshake_is_reported_expired_and_costs_nothing() {
    let config = LinkConfig {
        silence_penalty: Duration::from_secs(5),
        ..fast()
    };
    let (host, sender) = host_with(config).await;
    let addr = host.local_addr().unwrap();
    let start = Instant::now();
    let typed = shown_code(&sender);
    sender
        .lock()
        .unwrap()
        .tick(start + CODE_LIFETIME + Duration::from_secs(5))
        .unwrap();
    let serving = spawn_host(host, sender.clone());

    // The new laptop asks for its code (still in grace) and gets message 1 ...
    let mut raw = TcpStream::connect(addr).await.unwrap();
    raw.write_all(&[KIND_HELLO, 0, 1, typed.parity()])
        .await
        .unwrap();
    let (_, msg1) = read_frame(&mut raw).await;
    // ... and the grace ends before its reply arrives.
    sender
        .lock()
        .unwrap()
        .tick(start + CODE_LIFETIME + ROTATION_GRACE + Duration::from_secs(1))
        .unwrap();
    let (_r, msg2) = ReceiverSession::respond(&typed, &msg1, Instant::now()).unwrap();
    let mut frame = vec![1u8];
    frame.extend((msg2.len() as u16).to_be_bytes());
    frame.extend(&msg2);
    raw.write_all(&frame).await.unwrap();
    assert_eq!(read_frame(&mut raw).await.0, KIND_EXPIRED);

    // Not a guess, and the address is not refused: typing the new code pairs at once.
    assert_eq!(sender.lock().unwrap().failed_attempts(), 0);
    let fresh = shown_code(&sender);
    assert!(connect(addr, &fresh, config).await.is_ok());
    serving.abort();
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

/// Runs `misbehave` against a host whose penalty is 2 s, then checks a device at the same
/// address is refused at once, and that the slot was held no longer than the handshake limit.
async fn assert_penalised<F, Fut>(misbehave: F)
where
    F: FnOnce(std::net::SocketAddr, u8) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let config = LinkConfig {
        silence_penalty: Duration::from_secs(2),
        ..fast()
    };
    let (host, sender) = host_with(config).await;
    let addr = host.local_addr().unwrap();
    let code = shown_code(&sender);
    let serving = spawn_host(host, sender.clone());

    let started = Instant::now();
    misbehave(addr, code.parity()).await;
    assert!(
        started.elapsed() < Duration::from_millis(900),
        "held the slot for {:?}",
        started.elapsed()
    );
    // Give the host a moment to record the outcome.
    tokio::time::sleep(Duration::from_millis(50)).await;
    let err = connect(addr, &code, config).await.unwrap_err();
    assert!(matches!(err, LinkError::Busy), "got {err:?}");
    assert_eq!(sender.lock().unwrap().failed_attempts(), 0, "not a guess");
    serving.abort();
}

#[tokio::test]
async fn hanging_up_before_the_time_limit_still_costs_the_address() {
    assert_penalised(|addr, parity| async move {
        let mut raw = TcpStream::connect(addr).await.unwrap();
        raw.write_all(&[KIND_HELLO, 0, 1, parity]).await.unwrap();
        let _msg1 = read_frame(&mut raw).await;
        tokio::time::sleep(Duration::from_millis(300)).await;
        drop(raw); // hang up just before the limit
    })
    .await;
}

#[tokio::test]
async fn a_status_notice_sent_to_the_old_laptop_costs_the_address() {
    assert_penalised(|addr, parity| async move {
        let mut raw = TcpStream::connect(addr).await.unwrap();
        raw.write_all(&[KIND_HELLO, 0, 1, parity]).await.unwrap();
        let _msg1 = read_frame(&mut raw).await;
        raw.write_all(&[KIND_PAUSED, 0, 0]).await.unwrap();
        let mut sink = Vec::new();
        let _ = raw.read_to_end(&mut sink).await;
    })
    .await;
}

#[tokio::test]
async fn stalling_after_hello_is_cut_off_at_the_handshake_limit_and_costs_the_address() {
    assert_penalised(|addr, parity| async move {
        let mut raw = TcpStream::connect(addr).await.unwrap();
        raw.write_all(&[KIND_HELLO, 0, 1, parity]).await.unwrap();
        let _msg1 = read_frame(&mut raw).await;
        let mut sink = Vec::new();
        let _ = raw.read_to_end(&mut sink).await; // the host hangs up at the limit
    })
    .await;
}

#[tokio::test]
async fn a_reply_naming_another_session_costs_the_address() {
    // Otherwise it would look like an honestly expired code and cost nothing.
    assert_penalised(|addr, parity| async move {
        let mut raw = TcpStream::connect(addr).await.unwrap();
        raw.write_all(&[KIND_HELLO, 0, 1, parity]).await.unwrap();
        let (_, mut msg2) = read_frame(&mut raw).await;
        msg2[1] = 2;
        msg2[2] ^= 0xff; // a session tag nobody was given
        let mut frame = vec![1u8];
        frame.extend((msg2.len() as u16).to_be_bytes());
        frame.extend(&msg2);
        raw.write_all(&frame).await.unwrap();
        let mut sink = Vec::new();
        let _ = raw.read_to_end(&mut sink).await;
    })
    .await;
}

#[tokio::test]
async fn a_broken_reply_for_the_right_session_costs_the_address_and_a_guess() {
    let config = LinkConfig {
        silence_penalty: Duration::from_secs(5),
        ..fast()
    };
    let (host, sender) = host_with(config).await;
    let addr = host.local_addr().unwrap();
    let code = shown_code(&sender);
    let serving = spawn_host(host, sender.clone());

    let mut raw = TcpStream::connect(addr).await.unwrap();
    raw.write_all(&[KIND_HELLO, 0, 1, code.parity()])
        .await
        .unwrap();
    let (_, mut msg1) = read_frame(&mut raw).await;
    msg1[1] = 2;
    msg1.truncate(12); // the right session tag, then nothing usable
    let mut frame = vec![KIND_PAIRING];
    frame.extend((msg1.len() as u16).to_be_bytes());
    frame.extend(&msg1);
    raw.write_all(&frame).await.unwrap();
    let mut sink = Vec::new();
    let _ = raw.read_to_end(&mut sink).await;

    // It used up the code (one failure) and, once the pause is over, the address is refused.
    assert_eq!(sender.lock().unwrap().failed_attempts(), 1);
    tokio::time::sleep(Duration::from_millis(1100)).await;
    let fresh = shown_code(&sender);
    let err = connect(addr, &fresh, config).await.unwrap_err();
    assert!(matches!(err, LinkError::Busy), "got {err:?}");
    serving.abort();
}

#[tokio::test]
async fn a_device_that_dawdles_at_every_step_is_cut_off_at_the_whole_handshake_limit() {
    // Each step alone is within the limit; together they are not.
    let config = LinkConfig {
        silence_penalty: Duration::from_secs(5),
        ..fast()
    };
    let (host, sender) = host_with(config).await;
    let addr = host.local_addr().unwrap();
    let code = shown_code(&sender);
    let reached_pick = tokio::spawn({
        let sender = sender.clone();
        async move {
            tokio::time::timeout(Duration::from_millis(1500), host.next_peer(&sender))
                .await
                .is_ok()
        }
    });

    let mut raw = TcpStream::connect(addr).await.unwrap();
    raw.write_all(&[KIND_HELLO, 0, 1, code.parity()])
        .await
        .unwrap();
    let (_, msg1) = read_frame(&mut raw).await;
    tokio::time::sleep(Duration::from_millis(250)).await;
    let (receiver, msg2) = ReceiverSession::respond(&code, &msg1, Instant::now()).unwrap();
    let mut frame = vec![1u8];
    frame.extend((msg2.len() as u16).to_be_bytes());
    frame.extend(&msg2);
    raw.write_all(&frame).await.unwrap();
    let (_, msg3) = read_frame(&mut raw).await;
    tokio::time::sleep(Duration::from_millis(250)).await;
    let (_waiting, reveal) = receiver.receive(&msg3, Instant::now()).unwrap();
    let mut frame = vec![1u8];
    frame.extend((reveal.len() as u16).to_be_bytes());
    frame.extend(&reveal);
    let _ = raw.write_all(&frame).await;
    // Cut off by the whole-handshake limit costs the address, like any other stall.
    tokio::time::sleep(Duration::from_millis(50)).await;
    let fresh = shown_code(&sender);
    let err = connect(addr, &fresh, config).await.unwrap_err();
    assert!(matches!(err, LinkError::Busy), "got {err:?}");
    // The host hung up at the 400 ms limit, so this device never reached the number pick.
    assert!(
        !reached_pick.await.unwrap(),
        "a dawdling device must not reach the pick"
    );
}

#[tokio::test]
async fn an_expired_code_does_not_cost_the_address() {
    let config = LinkConfig {
        silence_penalty: Duration::from_secs(5),
        ..fast()
    };
    let (host, sender) = host_with(config).await;
    let addr = host.local_addr().unwrap();
    let shown = shown_code(&sender);
    let other_last = if shown.digits().as_bytes()[5].is_multiple_of(2) {
        '1'
    } else {
        '0'
    };
    let stale = PairingCode::parse(&format!("{}{other_last}", &shown.digits()[..5])).unwrap();
    let serving = spawn_host(host, sender.clone());
    let err = connect(addr, &stale, config).await.unwrap_err();
    assert!(matches!(err, LinkError::Pairing(PairingError::Expired)));
    // Typing the code shown now works straight away.
    assert!(connect(addr, &shown, config).await.is_ok());
    serving.abort();
}

#[tokio::test]
async fn asking_again_for_an_expired_code_costs_the_address() {
    let config = LinkConfig {
        silence_penalty: Duration::from_secs(5),
        ..fast()
    };
    let (host, sender) = host_with(config).await;
    let addr = host.local_addr().unwrap();
    let shown = shown_code(&sender).digits();
    let other_last = if shown.as_bytes()[5].is_multiple_of(2) {
        '1'
    } else {
        '0'
    };
    let stale = PairingCode::parse(&format!("{}{other_last}", &shown[..5])).unwrap();
    let serving = spawn_host(host, sender.clone());
    for _ in 0..2 {
        let err = connect(addr, &stale, config).await.unwrap_err();
        assert!(
            matches!(err, LinkError::Pairing(PairingError::Expired)),
            "got {err:?}"
        );
    }
    let err = connect(addr, &stale, config).await.unwrap_err();
    assert!(matches!(err, LinkError::Busy), "got {err:?}");
    serving.abort();
}
#[tokio::test]
async fn a_bad_hello_costs_the_address() {
    assert_penalised(|addr, _parity| async move {
        let mut raw = TcpStream::connect(addr).await.unwrap();
        raw.write_all(&[KIND_HELLO, 0, 1, 7]).await.unwrap();
        let mut sink = Vec::new();
        let _ = raw.read_to_end(&mut sink).await;
    })
    .await;
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
    // The listener survived; the real device pairs, trying again while it is told busy.
    let started = Instant::now();
    let pending_guest = loop {
        match connect(addr, &code, fast()).await {
            Ok(guest) => break guest,
            Err(LinkError::Busy) if started.elapsed() < Duration::from_secs(10) => {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            Err(e) => panic!("the real device was not served: {e:?}"),
        }
    };
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
    let started = Instant::now();
    let err = connect(addr, &wrong, fast()).await.unwrap_err();
    assert!(
        matches!(err, LinkError::Pairing(PairingError::HandshakeFailed)),
        "a mistyped code says so plainly; got {err:?}"
    );
    assert!(
        started.elapsed() < Duration::from_millis(300),
        "told at once"
    );
    assert_eq!(sender.lock().unwrap().failed_attempts(), 1);
    serving.abort();
}

#[tokio::test]
async fn a_mistyped_code_never_costs_the_address() {
    let config = LinkConfig {
        silence_penalty: Duration::from_secs(30),
        ..fast()
    };
    let (host, sender) = host_with(config).await;
    let addr = host.local_addr().unwrap();
    let wrong = wrong_code_for(&sender);
    let serving = tokio::spawn({
        let sender = sender.clone();
        async move { host.next_peer(&sender).await.map(|p| p.choices()) }
    });
    assert!(connect(addr, &wrong, config).await.is_err());
    // During the pause the same address is told to wait, not that the laptop is busy ...
    let err = connect(addr, &wrong, config).await.unwrap_err();
    assert!(
        matches!(err, LinkError::Pairing(PairingError::CoolingDown)),
        "got {err:?}"
    );
    // ... and after it, the person types the right code and pairs straight away.
    tokio::time::sleep(Duration::from_millis(1100)).await;
    let right = shown_code(&sender);
    let guest = connect(addr, &right, config).await.unwrap();
    let choices = serving.await.unwrap().unwrap();
    assert!(choices.contains(&guest.match_number()));
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

    let started = Instant::now();
    let err = connect(addr, &stale, fast()).await.unwrap_err();
    assert!(
        matches!(err, LinkError::Pairing(PairingError::Expired)),
        "got {err:?}"
    );
    assert!(
        started.elapsed() < Duration::from_millis(300),
        "told at once"
    );
    assert_eq!(sender.lock().unwrap().failed_attempts(), 0, "not a guess");
    serving.abort();
}

fn lock_by_five_guesses(sender: &Mutex<RotatingSender>) {
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

#[tokio::test]
async fn while_locked_every_device_is_told_at_once_until_the_person_starts_again() {
    let (host, sender) = host_and_sender().await;
    let addr = host.local_addr().unwrap();
    lock_by_five_guesses(&sender);
    let serving = tokio::spawn({
        let sender = sender.clone();
        async move { host.next_peer(&sender).await.map(|p| p.choices()) }
    });

    let code = PairingCode::parse("123456").unwrap();
    for _ in 0..3 {
        let started = Instant::now();
        let err = connect(addr, &code, fast()).await.unwrap_err();
        assert!(
            matches!(err, LinkError::Pairing(PairingError::Locked)),
            "got {err:?}"
        );
        assert!(
            started.elapsed() < Duration::from_millis(300),
            "told at once"
        );
    }

    // The person chooses Start again; the next device pairs.
    sender.lock().unwrap().unlock(Instant::now()).unwrap();
    let fresh = shown_code(&sender);
    let guest = connect(addr, &fresh, fast()).await.unwrap();
    let choices = serving.await.unwrap().unwrap();
    assert!(choices.contains(&guest.match_number()));
}

#[tokio::test]
async fn the_guess_that_triggers_the_safety_stop_is_told_locked() {
    let (host, sender) = host_and_sender().await;
    let addr = host.local_addr().unwrap();
    {
        // Four failures so far; the fifth will lock.
        let mut s = sender.lock().unwrap();
        // Spaced 10 s apart and ending 10 s ago, so every pause (1, 2, 4, 8 s) is over now.
        let start = Instant::now() - Duration::from_secs(40);
        for i in 0..4u64 {
            let t = start + Duration::from_secs(10 * i);
            s.tick(t).unwrap();
            let mut junk = vec![1u8, 2];
            junk.extend(&s.message_1().unwrap()[2..10]);
            junk.extend([7u8; 60]);
            assert!(s.receive(&junk, t).is_err());
        }
        s.tick(Instant::now()).unwrap();
    }
    let wrong = wrong_code_for(&sender);
    let serving = spawn_host(host, sender.clone());
    let err = connect(addr, &wrong, fast()).await.unwrap_err();
    assert!(
        matches!(err, LinkError::Pairing(PairingError::Locked)),
        "got {err:?}"
    );
    serving.abort();
}

#[tokio::test]
async fn after_a_send_times_out_the_link_refuses_further_use() {
    let (host, sender) = host_and_sender().await;
    let addr = host.local_addr().unwrap();
    let code = shown_code(&sender);
    let guest = tokio::spawn(async move { connect(addr, &code, fast()).await.unwrap() });
    let pending_host = host.next_peer(&sender).await.unwrap();
    let pending_guest = guest.await.unwrap();
    let shown = pending_guest.match_number();
    let mut host_link = pending_host.choose(shown, Instant::now()).await.unwrap();
    let _guest_link = pending_guest.approval().await.unwrap(); // never reads

    let big = vec![0u8; 60_000];
    let mut timed_out = false;
    for _ in 0..200 {
        if let Err(e) = host_link.send(&big).await {
            assert!(matches!(e, LinkError::Timeout), "got {e:?}");
            timed_out = true;
            break;
        }
    }
    assert!(timed_out, "the other laptop stopped reading");
    // A half-written frame would corrupt the stream, so the link is now closed for good.
    assert!(matches!(
        host_link.send(b"more").await,
        Err(LinkError::Closed)
    ));
}

async fn paired_links() -> (pctwin_link::Link, pctwin_link::Link) {
    let (host, sender) = host_and_sender().await;
    let addr = host.local_addr().unwrap();
    let code = shown_code(&sender);
    let guest = tokio::spawn(async move { connect(addr, &code, fast()).await.unwrap() });
    let pending_host = host.next_peer(&sender).await.unwrap();
    let pending_guest = guest.await.unwrap();
    let shown = pending_guest.match_number();
    let host_link = pending_host.choose(shown, Instant::now()).await.unwrap();
    let guest_link = pending_guest.approval().await.unwrap();
    (host_link, guest_link)
}

#[tokio::test]
async fn a_cancelled_receive_closes_the_link_for_both_directions() {
    let (mut host_link, mut guest_link) = paired_links().await;
    // The app gives up waiting part-way: the read may have consumed part of a frame.
    assert!(
        tokio::time::timeout(Duration::from_millis(50), guest_link.recv())
            .await
            .is_err()
    );
    host_link.send(b"arrives after the give-up").await.unwrap();
    assert!(matches!(guest_link.recv().await, Err(LinkError::Closed)));
    assert!(matches!(
        guest_link.send(b"x").await,
        Err(LinkError::Closed)
    ));
}

#[tokio::test]
async fn a_failed_receive_closes_the_link_for_both_directions() {
    let (host_link, mut guest_link) = paired_links().await;
    drop(host_link); // the old laptop goes away
    assert!(guest_link.recv().await.is_err());
    assert!(matches!(
        guest_link.send(b"x").await,
        Err(LinkError::Closed)
    ));
    assert!(matches!(guest_link.recv().await, Err(LinkError::Closed)));
}

#[tokio::test]
async fn a_cancelled_send_closes_the_link_for_both_directions() {
    let (mut host_link, _guest_link) = paired_links().await; // the guest never reads
    let big = vec![0u8; 60_000];
    // Fill the buffers, then give up on a send part-way, as an app timeout would.
    let mut cancelled = false;
    for _ in 0..200 {
        if tokio::time::timeout(Duration::from_millis(50), host_link.send(&big))
            .await
            .is_err()
        {
            cancelled = true;
            break;
        }
    }
    assert!(cancelled, "a send was left half-done");
    assert!(matches!(host_link.send(b"x").await, Err(LinkError::Closed)));
    assert!(matches!(host_link.recv().await, Err(LinkError::Closed)));
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

// ---------- fairness: a misbehaving device cannot keep a real one from pairing ----------

/// How one misbehaving device at 127.0.0.2 uses each connection.
#[derive(Clone, Copy, Debug)]
enum Misbehaviour {
    HangUpJustBeforeTheLimit,
    SendANoticeInsteadOfAReply,
    StallAfterHello,
    SendABadHello,
    ReplyForAnotherSession,
    AskForAnExpiredCode,
    /// Knows the code (saw the screen) and is slow at every step, never letting the person pick.
    DawdleWithTheCode,
}

async fn misbehave_once(
    addr: std::net::SocketAddr,
    how: Misbehaviour,
    parity: u8,
    sender: &Mutex<RotatingSender>,
) {
    let code = sender.lock().unwrap().code();
    let socket = tokio::net::TcpSocket::new_v4().unwrap();
    socket.bind("127.0.0.2:0".parse().unwrap()).unwrap();
    let Ok(mut raw) = socket.connect(addr).await else {
        return;
    };
    let hello_parity = match how {
        Misbehaviour::SendABadHello => 9,
        Misbehaviour::AskForAnExpiredCode => 1 - parity,
        Misbehaviour::DawdleWithTheCode => match &code {
            Some(c) => PairingCode::parse(c).unwrap().parity(),
            None => return,
        },
        _ => parity,
    };
    if raw
        .write_all(&[KIND_HELLO, 0, 1, hello_parity])
        .await
        .is_err()
    {
        return;
    }
    let mut header = [0u8; 3];
    if raw.read_exact(&mut header).await.is_err() || header[0] != KIND_PAIRING {
        return; // refused, told expired, or closed
    }
    let mut msg1 = vec![0u8; u16::from_be_bytes([header[1], header[2]]) as usize];
    if raw.read_exact(&mut msg1).await.is_err() {
        return;
    }
    match how {
        Misbehaviour::HangUpJustBeforeTheLimit => {
            tokio::time::sleep(Duration::from_millis(350)).await;
            return; // hangs up itself, before the host's 400 ms limit
        }
        Misbehaviour::SendANoticeInsteadOfAReply => {
            tokio::time::sleep(Duration::from_millis(350)).await;
            let _ = raw.write_all(&[KIND_PAUSED, 0, 0]).await;
        }
        Misbehaviour::DawdleWithTheCode => {
            let Some(code) = code.and_then(|c| PairingCode::parse(&c).ok()) else {
                return;
            };
            tokio::time::sleep(Duration::from_millis(250)).await;
            let Ok((receiver, msg2)) = ReceiverSession::respond(&code, &msg1, Instant::now())
            else {
                return;
            };
            let mut frame = vec![KIND_PAIRING];
            frame.extend((msg2.len() as u16).to_be_bytes());
            frame.extend(&msg2);
            if raw.write_all(&frame).await.is_err() {
                return;
            }
            let mut header = [0u8; 3];
            if raw.read_exact(&mut header).await.is_err() || header[0] != KIND_PAIRING {
                return;
            }
            let mut msg3 = vec![0u8; u16::from_be_bytes([header[1], header[2]]) as usize];
            if raw.read_exact(&mut msg3).await.is_err() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
            if let Ok((_waiting, reveal)) = receiver.receive(&msg3, Instant::now()) {
                let mut frame = vec![KIND_PAIRING];
                frame.extend((reveal.len() as u16).to_be_bytes());
                frame.extend(&reveal);
                let _ = raw.write_all(&frame).await;
            }
        }
        Misbehaviour::ReplyForAnotherSession => {
            msg1[1] = 2;
            msg1[2] ^= 0xff;
            let mut frame = vec![KIND_PAIRING];
            frame.extend((msg1.len() as u16).to_be_bytes());
            frame.extend(&msg1);
            let _ = raw.write_all(&frame).await;
        }
        _ => {}
    }
    let mut sink = Vec::new();
    let _ = tokio::time::timeout(Duration::from_millis(600), raw.read_to_end(&mut sink)).await;
}

/// The misbehaving device reconnects as fast as it can; the real device must still pair soon.
async fn real_device_pairs_despite(how: Misbehaviour) {
    let config = LinkConfig {
        silence_penalty: Duration::from_secs(30),
        ..fast()
    };
    let (host, sender) = host_with(config).await;
    let addr = host.local_addr().unwrap();
    let parity = shown_code(&sender).parity();
    let serving = tokio::spawn({
        let sender = sender.clone();
        async move { host.next_peer(&sender).await.map(|p| p.choices()) }
    });
    // Eight connections at once, each reconnecting as soon as it is dropped or refused, so the
    // slot is always contended. These end-to-end checks catch a broken penalty most of the time,
    // not always (the real device can win a gap); the `*_costs_the_address` tests are the
    // deterministic guard for each rule.
    let pests: Vec<_> = (0..8)
        .map(|_| {
            let sender = sender.clone();
            tokio::spawn(async move {
                loop {
                    misbehave_once(addr, how, parity, &sender).await;
                }
            })
        })
        .collect();
    tokio::time::sleep(Duration::from_millis(100)).await; // let it get going first

    let started = Instant::now();
    let paired = loop {
        // Read the code each time: a pest that knows the code uses it up.
        let code = sender
            .lock()
            .unwrap()
            .code()
            .map(|c| PairingCode::parse(&c).unwrap());
        let Some(code) = code else {
            tokio::time::sleep(Duration::from_millis(100)).await;
            continue;
        };
        match connect(addr, &code, config).await {
            Ok(guest) => break Some(guest),
            Err(_) if started.elapsed() < Duration::from_secs(4) => {
                tokio::time::sleep(Duration::from_millis(300)).await;
            }
            Err(_) => break None,
        }
    };
    for p in pests {
        p.abort();
    }
    let guest = paired.unwrap_or_else(|| panic!("{how:?} kept the real device out for 4 s"));
    let choices = serving.await.unwrap().unwrap();
    assert!(choices.contains(&guest.match_number()));
    assert_eq!(sender.lock().unwrap().failed_attempts(), 0, "{how:?}");
}

#[tokio::test]
async fn hanging_up_just_before_the_limit_cannot_keep_a_real_device_out() {
    real_device_pairs_despite(Misbehaviour::HangUpJustBeforeTheLimit).await;
}

#[tokio::test]
async fn sending_notices_cannot_keep_a_real_device_out() {
    real_device_pairs_despite(Misbehaviour::SendANoticeInsteadOfAReply).await;
}

#[tokio::test]
async fn stalling_after_hello_cannot_keep_a_real_device_out() {
    real_device_pairs_despite(Misbehaviour::StallAfterHello).await;
}

#[tokio::test]
async fn bad_hellos_cannot_keep_a_real_device_out() {
    real_device_pairs_despite(Misbehaviour::SendABadHello).await;
}

#[tokio::test]
async fn replies_for_other_sessions_cannot_keep_a_real_device_out() {
    real_device_pairs_despite(Misbehaviour::ReplyForAnotherSession).await;
}

#[tokio::test]
async fn asking_again_and_again_for_an_expired_code_cannot_keep_a_real_device_out() {
    // The first expired answer is free; asking again soon after costs the address.
    real_device_pairs_despite(Misbehaviour::AskForAnExpiredCode).await;
}

#[tokio::test]
async fn a_code_holder_dawdling_at_every_step_cannot_keep_a_real_device_out() {
    real_device_pairs_despite(Misbehaviour::DawdleWithTheCode).await;
}

#[tokio::test]
async fn a_refused_connection_is_reported_unreachable_even_when_it_takes_a_while() {
    // On Windows a refused connection takes about two seconds; allow for it.
    let spare = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = spare.local_addr().unwrap();
    drop(spare);
    let patient = LinkConfig {
        connect_timeout: Duration::from_secs(8),
        ..fast()
    };
    let code = PairingCode::parse("123456").unwrap();
    let err = connect(addr, &code, patient).await.unwrap_err();
    assert!(matches!(err, LinkError::Unreachable), "got {err:?}");
}

#[tokio::test]
async fn the_connect_limit_not_the_step_limit_decides_how_long_an_address_may_take() {
    // An address that never answers (reserved for documentation, not routed).
    let silent: std::net::SocketAddr = "192.0.2.1:9".parse().unwrap();
    let config = LinkConfig {
        step_timeout: Duration::from_secs(20),
        connect_timeout: Duration::from_millis(300),
        ..fast()
    };
    let code = PairingCode::parse("123456").unwrap();
    let started = Instant::now();
    let err = connect(silent, &code, config).await.unwrap_err();
    assert!(matches!(err, LinkError::Unreachable), "got {err:?}");
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "{:?}",
        started.elapsed()
    );
}
