//! Acceptance tests for pairing over a real network connection (Security Design Part A).
//!
//! The old laptop (host) listens; the new laptop (guest) connects after the person typed the code.
//! Only one guest can be pairing at a time, so a reply from anyone else on the network can never
//! spend the attempt of the guest in progress.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use pctwin_link::{Host, LinkConfig, LinkError, connect};
use pctwin_pairing::{PairingCode, PairingError, RotatingSender};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

fn fast() -> LinkConfig {
    LinkConfig {
        step_timeout: Duration::from_millis(400),
    }
}

async fn host_and_sender() -> (Host, Mutex<RotatingSender>) {
    let host = Host::bind("127.0.0.1:0".parse().unwrap(), fast())
        .await
        .unwrap();
    let sender = Mutex::new(RotatingSender::new(Instant::now()).unwrap());
    (host, sender)
}

fn shown_code(sender: &Mutex<RotatingSender>) -> PairingCode {
    PairingCode::parse(&sender.lock().unwrap().code().unwrap()).unwrap()
}

#[tokio::test]
async fn two_laptops_pair_over_a_real_connection_and_exchange_sealed_data() {
    let (host, sender) = host_and_sender().await;
    let addr = host.local_addr().unwrap();
    let code = shown_code(&sender);

    let guest = tokio::spawn(async move {
        let pending = connect(addr, &code, fast()).await.unwrap();
        let shown = pending.match_number();
        (shown, pending)
    });
    let pending_host = host.next_peer(&sender).await.unwrap();
    let (shown, pending_guest) = guest.await.unwrap();

    // The person reads the new laptop's number and picks it on the old laptop.
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
async fn while_one_device_is_pairing_any_other_is_told_busy_and_cannot_touch_it() {
    let (host, sender) = host_and_sender().await;
    let addr = host.local_addr().unwrap();
    // A first device holds the slot (it received message 1 and is "typing").
    let mut first = TcpStream::connect(addr).await.unwrap();
    let serving = tokio::spawn(async move {
        let _ = host.next_peer(&sender).await;
    });
    let mut header = [0u8; 3];
    first.read_exact(&mut header).await.unwrap();
    assert_eq!(header[0], 1, "the first device gets message 1");

    // A second device tries to slip a reply in; it is told busy and closed without being read.
    let mut raw = TcpStream::connect(addr).await.unwrap();
    let _ = raw.write_all(&[1, 0, 4, 1, 2, 3, 4]).await;
    let mut reply = Vec::new();
    let _ = raw.read_to_end(&mut reply).await;
    assert_eq!(reply, vec![2, 0, 0], "the second device is told busy");

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
        let _ = raw.read_to_end(&mut sink).await; // receives message 1, then says nothing
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
async fn an_oversized_message_is_refused_and_the_next_device_pairs() {
    let (host, sender) = host_and_sender().await;
    let addr = host.local_addr().unwrap();
    let code = shown_code(&sender);

    let flooder = tokio::spawn(async move {
        let mut raw = TcpStream::connect(addr).await.unwrap();
        let mut header = [0u8; 3];
        raw.read_exact(&mut header).await.unwrap();
        let len = u16::from_be_bytes([header[1], header[2]]) as usize;
        let mut msg1 = vec![0u8; len];
        raw.read_exact(&mut msg1).await.unwrap();
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
async fn a_wrong_code_fails_for_the_guest_and_counts_as_one_guess() {
    let (host, sender) = host_and_sender().await;
    let addr = host.local_addr().unwrap();
    let right = shown_code(&sender).digits();
    let wrong = if right == "000000" {
        "000001"
    } else {
        "000000"
    };
    let wrong = PairingCode::parse(wrong).unwrap();

    let sender = std::sync::Arc::new(sender);
    let serving = tokio::spawn({
        let sender = sender.clone();
        async move {
            let _ = host.next_peer(&sender).await;
        }
    });
    assert!(connect(addr, &wrong, fast()).await.is_err());
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(sender.lock().unwrap().failed_attempts(), 1);
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
