//! Sessions between remembered connections (Security Design Part T), over a real network
//! connection: no code, a Noise `KK` handshake with the keys both laptops remembered
//! ([`pctwin_pairing::known`]), then the same sealed [`Link`] as a paired session.
//!
//! - The laptop that connects ([`connect_known`]) first says which connection it means by its
//!   short, often-changing token ([`HINT_LEN`] bytes, derived from the pair's secret by the
//!   caller): the only thing said in the clear, and something anyone nearby already saw announced.
//!   The token is bound into the handshake, so a session started for one token is not taken for
//!   another.
//! - The laptop that answers ([`KnownListener`]) serves only devices with local-network
//!   addresses, looks the token up in its own remembered connections (an unknown token is closed
//!   at once, with nothing said), and answers with the remembered key. The link is handed out only
//!   after the starter's sealed confirmation, so a replayed first message never becomes a link.
//! - Each step has [`pctwin_pairing::known::KNOWN_TIMEOUT`]; a device that stays silent is
//!   closed. The caller runs each [`KnownArrival::handshake`] on its own task, so a slow device
//!   never holds up the next.

use std::io;
use std::net::SocketAddr;
use std::time::Duration;

use pctwin_pairing::known::{KNOWN_TIMEOUT, StaticKeys, accept_known, open_known};
use tokio::net::{TcpListener, TcpStream};

use crate::{
    KIND_PAIRING, Link, LinkConfig, LinkError, MAX_LISTEN_ADDRS, is_local_peer, keep_alive, now,
    read_timed, write_frame,
};

/// The token a connecting laptop names its connection by.
pub const HINT_LEN: usize = 8;
/// A known-keys hello: `[hint][KK message 1]`.
const KIND_KNOWN_HELLO: u8 = 14;
/// Largest known-keys handshake frame.
const MAX_KNOWN_FRAME: usize = pctwin_pairing::MAX_MESSAGE_LEN + HINT_LEN;

/// What the answering laptop needs for a connection it remembers.
pub struct Remembered<C> {
    /// The caller's own name for the connection (handed back with the link).
    pub connection: C,
    /// This laptop's own keys for that connection.
    pub local: StaticKeys,
    /// The other laptop's remembered public key.
    pub remote: [u8; 32],
}

/// The laptop that connects: opens a session to the laptop at `addr` for the connection whose
/// token now is `hint`, with this side's `local` keys and the other's remembered `remote` key.
/// Fails with [`LinkError::Unreachable`] only if nothing was sent.
pub async fn connect_known(
    addr: SocketAddr,
    hint: &[u8; HINT_LEN],
    local: &StaticKeys,
    remote: &[u8; 32],
    config: LinkConfig,
) -> Result<Link, LinkError> {
    if !is_local_peer(addr.ip()) {
        return Err(LinkError::Unreachable);
    }
    let wait = config.step_timeout.min(KNOWN_TIMEOUT);
    let mut stream = tokio::time::timeout(config.connect_timeout, TcpStream::connect(addr))
        .await
        .map_err(|_| LinkError::Unreachable)?
        .map_err(|_| LinkError::Unreachable)?;
    keep_alive(&stream);
    let (opening, m1) = open_known(local, remote, hint, now())?;
    let mut hello = Vec::with_capacity(HINT_LEN + m1.len());
    hello.extend_from_slice(hint);
    hello.extend_from_slice(&m1);
    write_frame(&mut stream, KIND_KNOWN_HELLO, &hello, wait).await?;
    let (kind, m2) = read_timed(&mut stream, wait).await?;
    if kind != KIND_PAIRING {
        return Err(LinkError::Unexpected);
    }
    let (transport, m3) = opening.finish(&m2, now())?;
    write_frame(&mut stream, KIND_PAIRING, &m3, wait).await?;
    Ok(Link::known(transport, stream, addr, config.step_timeout))
}

/// Listens for remembered connections on this laptop's own addresses.
pub struct KnownListener {
    listeners: Vec<TcpListener>,
    config: LinkConfig,
}

impl std::fmt::Debug for KnownListener {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("KnownListener(..)")
    }
}

/// A device that connected, before its handshake.
pub struct KnownArrival {
    stream: TcpStream,
    peer: SocketAddr,
    config: LinkConfig,
}

impl std::fmt::Debug for KnownArrival {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("KnownArrival(..)")
    }
}

impl KnownListener {
    /// Listens on each of `addrs` (at most one IPv4 and one IPv6 address of the chosen network),
    /// all on the first one's port (port 0: any free port).
    pub async fn bind(addrs: &[SocketAddr], config: LinkConfig) -> io::Result<Self> {
        if addrs.is_empty() || addrs.len() > MAX_LISTEN_ADDRS {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "one IPv4 and one IPv6 address at most",
            ));
        }
        let mut listeners = vec![TcpListener::bind(addrs[0]).await?];
        let port = listeners[0].local_addr()?.port();
        for addr in &addrs[1..] {
            let mut at = *addr;
            at.set_port(port);
            listeners.push(TcpListener::bind(at).await?);
        }
        Ok(Self { listeners, config })
    }

    /// The port to announce.
    pub fn port(&self) -> io::Result<u16> {
        Ok(self.listeners[0].local_addr()?.port())
    }

    /// The next device with a local-network address; others are closed unread.
    pub async fn accept(&self) -> io::Result<KnownArrival> {
        loop {
            let (stream, peer) = match self.listeners.as_slice() {
                [only] => only.accept().await?,
                [first, second, ..] => tokio::select! {
                    a = first.accept() => a?,
                    b = second.accept() => b?,
                },
                [] => std::future::pending().await,
            };
            if is_local_peer(peer.ip()) {
                keep_alive(&stream);
                return Ok(KnownArrival {
                    stream,
                    peer,
                    config: self.config,
                });
            }
        }
    }
}

impl KnownArrival {
    /// Where the device connected from.
    pub fn peer_addr(&self) -> SocketAddr {
        self.peer
    }

    /// Runs the answering side of the handshake: reads the hello, asks `lookup` for the
    /// connection its token names (`None`: closed at once, nothing said), answers with that
    /// connection's keys, and hands out the link only after the starter's confirmation.
    pub async fn handshake<C>(
        mut self,
        lookup: impl FnOnce(&[u8; HINT_LEN]) -> Option<Remembered<C>>,
    ) -> Result<(C, Link), LinkError> {
        let wait: Duration = self.config.step_timeout.min(KNOWN_TIMEOUT);
        let (kind, hello) =
            tokio::time::timeout(wait, crate::read_frame(&mut self.stream, MAX_KNOWN_FRAME))
                .await
                .map_err(|_| LinkError::Timeout)??;
        if kind != KIND_KNOWN_HELLO || hello.len() <= HINT_LEN {
            return Err(LinkError::Unexpected);
        }
        let (hint, m1) = hello.split_at(HINT_LEN);
        let hint: [u8; HINT_LEN] = hint.try_into().map_err(|_| LinkError::Unexpected)?;
        let known = lookup(&hint).ok_or(LinkError::Closed)?;
        let (accepting, m2) = accept_known(&known.local, &known.remote, &hint, m1, now())?;
        write_frame(&mut self.stream, KIND_PAIRING, &m2, wait).await?;
        let (kind, m3) = read_timed(&mut self.stream, wait).await?;
        if kind != KIND_PAIRING {
            return Err(LinkError::Unexpected);
        }
        let transport = accepting.confirm(&m3, now())?;
        Ok((
            known.connection,
            Link::known(transport, self.stream, self.peer, self.config.step_timeout),
        ))
    }
}
