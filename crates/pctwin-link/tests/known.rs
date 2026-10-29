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
        let arrival = l.accept().await.unwrap();
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
    let answer = async {
        l.accept()
            .await
            .unwrap()
            .handshake(|_| None::<Remembered<()>>)
            .await
    };
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
            .unwrap()
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
    let got = l
        .accept()
        .await
        .unwrap()
        .handshake(|_| None::<Remembered<()>>)
        .await;
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
