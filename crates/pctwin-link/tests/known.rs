//! Remembered connections over a real network connection (Security Design Part T).

use std::net::SocketAddr;
use std::time::Duration;

use pctwin_link::known::{HINT_LEN, KnownListener, Remembered, connect_known};
use pctwin_link::{LinkConfig, LinkError};
use pctwin_pairing::known::StaticKeys;
use zeroize::Zeroizing;

fn copy(k: &StaticKeys) -> StaticKeys {
    StaticKeys::from_private(Zeroizing::new(**k.private()))
}

async fn listener() -> (KnownListener, SocketAddr) {
    let l = KnownListener::bind(&["127.0.0.1:0".parse().unwrap()], LinkConfig::default())
        .await
        .unwrap();
    let port = l.port().unwrap();
    (l, SocketAddr::from(([127, 0, 0, 1], port)))
}

const HINT: [u8; HINT_LEN] = *b"tok12345";

#[tokio::test]
async fn a_remembered_connection_gets_a_sealed_link_without_a_code() {
    let (a, b) = (
        StaticKeys::generate().unwrap(),
        StaticKeys::generate().unwrap(),
    );
    let (l, addr) = listener().await;
    let (a_pub, b_pub) = (a.public(), b.public());
    let b2 = copy(&b);
    let answer = async {
        let arrival = l.accept().await;
        arrival
            .handshake(|hint| {
                (hint == &HINT).then(|| Remembered {
                    connection: "alex",
                    local: b2,
                    remote: a_pub,
                })
            })
            .await
    };
    let start = connect_known(addr, &HINT, &a, &b_pub, LinkConfig::default());
    let (answered, started) = tokio::join!(answer, start);
    let (who, mut lb) = answered.unwrap();
    let mut la = started.unwrap();
    assert_eq!(who, "alex");
    la.send(b"hello").await.unwrap();
    assert_eq!(lb.recv().await.unwrap().as_slice(), b"hello");
    lb.send(b"back").await.unwrap();
    assert_eq!(la.recv().await.unwrap().as_slice(), b"back");
}

#[tokio::test]
async fn an_unknown_token_is_closed_with_nothing_said() {
    let (a, b) = (
        StaticKeys::generate().unwrap(),
        StaticKeys::generate().unwrap(),
    );
    let (l, addr) = listener().await;
    let answer = async { l.accept().await.handshake(|_| None::<Remembered<()>>).await };
    let b_pub = b.public();
    let start = connect_known(addr, &HINT, &a, &b_pub, LinkConfig::default());
    let (answered, started) = tokio::join!(answer, start);
    assert!(matches!(answered, Err(LinkError::Closed)));
    assert!(started.is_err());
}

/// A laptop that answers with another key (a stranger at the remembered address) is refused.
#[tokio::test]
async fn a_stranger_at_the_address_is_refused() {
    let (a, b) = (
        StaticKeys::generate().unwrap(),
        StaticKeys::generate().unwrap(),
    );
    let stranger = StaticKeys::generate().unwrap();
    let (l, addr) = listener().await;
    let a_pub = a.public();
    let answer = async {
        l.accept()
            .await
            .handshake(|_| {
                Some(Remembered {
                    connection: (),
                    local: stranger,
                    remote: a_pub,
                })
            })
            .await
    };
    let b_pub = b.public();
    let start = connect_known(addr, &HINT, &a, &b_pub, LinkConfig::default());
    let (answered, started) = tokio::join!(answer, start);
    assert!(answered.is_err());
    assert!(started.is_err());
}

/// A device that connects and says nothing is closed after the step limit.
#[tokio::test]
async fn a_silent_device_is_closed() {
    let config = LinkConfig {
        step_timeout: Duration::from_millis(200),
        ..LinkConfig::default()
    };
    let l = KnownListener::bind(&["127.0.0.1:0".parse().unwrap()], config)
        .await
        .unwrap();
    let addr2 = SocketAddr::from(([127, 0, 0, 1], l.port().unwrap()));
    let _silent = tokio::net::TcpStream::connect(addr2).await.unwrap();
    let got = l.accept().await.handshake(|_| None::<Remembered<()>>).await;
    assert!(matches!(got, Err(LinkError::Timeout)), "{got:?}");
}

#[tokio::test]
async fn only_local_network_addresses_are_dialled() {
    let (a, b) = (
        StaticKeys::generate().unwrap(),
        StaticKeys::generate().unwrap(),
    );
    let far: SocketAddr = "8.8.8.8:4000".parse().unwrap();
    let got = connect_known(far, &HINT, &a, &b.public(), LinkConfig::default()).await;
    assert!(matches!(got, Err(LinkError::Unreachable)));
}

/// Anyone nearby can connect: at most so many handshakes at once, and so many arrivals from one
/// address per window; the rest are closed unread, and a place is given back when a handshake
/// ends.
#[test]
fn the_gate_caps_handshakes_at_once_and_arrivals_per_address() {
    use pctwin_link::known::{ArrivalGate, RATE_WINDOW};
    use std::net::IpAddr;
    use std::time::Instant;
    let t0 = Instant::now();
    let a: IpAddr = "192.168.1.20".parse().unwrap();
    let b: IpAddr = "192.168.1.21".parse().unwrap();
    let gate = ArrivalGate::new(2, 3);
    let one = gate.enter(a, t0).unwrap();
    let two = gate.enter(b, t0).unwrap();
    assert!(gate.enter(b, t0).is_none(), "two at once at most");
    assert_eq!(gate.running(), 2);
    drop(one);
    drop(two);
    assert_eq!(gate.running(), 0);
    // `a` has used one of its three; two more, then it waits for the next window.
    drop(gate.enter(a, t0).unwrap());
    drop(gate.enter(a, t0).unwrap());
    assert!(
        gate.enter(a, t0).is_none(),
        "three per window from one address"
    );
    assert!(
        gate.enter(b, t0).is_some(),
        "another address is not held up"
    );
    assert!(gate.enter(a, t0 + RATE_WINDOW).is_some(), "a new window");
}

#[test]
fn a_failing_accept_pauses_longer_each_time_up_to_a_limit() {
    use pctwin_link::known::accept_pause;
    assert!(accept_pause(1) < accept_pause(2));
    assert!(accept_pause(2) < accept_pause(3));
    assert_eq!(accept_pause(40), accept_pause(1000));
    assert!(accept_pause(1000) <= Duration::from_secs(2));
    assert!(accept_pause(1) >= Duration::from_millis(10));
}

/// A device that floods hellos is closed unread once over its rate, and the listener keeps
/// answering: a remembered laptop still gets its link afterwards.
#[tokio::test]
async fn a_flood_of_hellos_from_one_address_is_closed_unread() {
    use pctwin_link::known::ArrivalGate;
    use tokio::io::AsyncReadExt;
    let l = KnownListener::bind(&["127.0.0.1:0".parse().unwrap()], LinkConfig::default())
        .await
        .unwrap()
        .with_gate(ArrivalGate::new(4, 2));
    let addr = SocketAddr::from(([127, 0, 0, 1], l.port().unwrap()));
    // Two arrivals are let in (and held, unanswered); the third is closed at once.
    let serve = async {
        let first = l.accept().await;
        let second = l.accept().await;
        (first, second)
    };
    let connect = async {
        let s1 = tokio::net::TcpStream::connect(addr).await.unwrap();
        let s2 = tokio::net::TcpStream::connect(addr).await.unwrap();
        (s1, s2)
    };
    let ((_f, _s), (_s1, _s2)) = tokio::join!(serve, connect);
    let mut third = tokio::net::TcpStream::connect(addr).await.unwrap();
    // The listener accepts and drops it while waiting for an arrival it may let in.
    let waiting = tokio::time::timeout(Duration::from_millis(500), l.accept()).await;
    assert!(waiting.is_err(), "nothing let in from that address");
    let mut buf = [0u8; 1];
    let read = tokio::time::timeout(Duration::from_secs(5), third.read(&mut buf)).await;
    assert!(
        matches!(read, Ok(Ok(0)) | Ok(Err(_))),
        "closed unread: {read:?}"
    );
}
