//! PCTwin link: runs pairing over a real network connection and then carries sealed data
//! (Security Design, Part A).
//!
//! # How it works
//!
//! - The old laptop is the [`Host`]: it listens on a TCP port while pairing and serves one device
//!   at a time. Any other device that connects meanwhile is told the old laptop is busy and is
//!   closed without anything it sent being read, so nobody else on the network can spend or
//!   disturb the attempt in progress.
//! - The new laptop calls [`connect`] after the person typed the code. It receives the current
//!   message 1 on its own connection and answers straight away.
//! - Each step waits at most [`LinkConfig::step_timeout`]. A device that goes quiet is dropped and
//!   does not count as a guess; a wrong code counts against the old laptop's failure budget.
//! - After the person's pick on the old laptop, both sides get a [`Link`] that seals every message.
//!
//! Every frame on the wire is `[kind][length, 2 bytes big-endian][body]`. Pairing frames are
//! limited to [`MAX_MESSAGE_LEN`]; anything larger ends that connection before it is read.
//!
//! # Known limits
//!
//! - While the person is choosing the number, the host is not accepting: devices that connect then
//!   wait in the operating system's queue and are served on the next [`Host::next_peer`] call.
//! - A device on the network can still hold the single slot for one step timeout at a time, or
//!   spend the current code with a wrong guess. Both are limited (the timeout, the failure budget
//!   and lockout), not prevented.

use std::fmt;
use std::io;
use std::net::SocketAddr;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use pctwin_pairing::{
    MAX_MESSAGE_LEN, Paired, PairingCode, PairingError, ReceiverAwaitingApproval, ReceiverSession,
    RotatingSender, SenderChoosing, SenderStatus,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use zeroize::Zeroizing;

/// Default longest wait for each pairing step from the other laptop.
pub const STEP_TIMEOUT: Duration = Duration::from_secs(10);

const KIND_PAIRING: u8 = 1;
const KIND_BUSY: u8 = 2;
const KIND_DATA: u8 = 3;
const HEADER_LEN: usize = 3;
const MAX_DATA_LEN: usize = u16::MAX as usize;
/// How long a refused device gets to read the busy notice before it is closed.
const REFUSE_LINGER: Duration = Duration::from_secs(1);

/// Timing for one link.
#[derive(Debug, Clone, Copy)]
pub struct LinkConfig {
    /// Longest wait for each pairing step from the other laptop.
    pub step_timeout: Duration,
}

impl Default for LinkConfig {
    fn default() -> Self {
        Self {
            step_timeout: STEP_TIMEOUT,
        }
    }
}

/// Why connecting, pairing or the link failed.
#[derive(Debug, thiserror::Error)]
pub enum LinkError {
    #[error("the old laptop is pairing with another device; try again in a moment")]
    Busy,
    #[error("the other laptop stopped responding")]
    Timeout,
    #[error("the connection to the other laptop closed")]
    Closed,
    #[error("the other laptop sent a message that was too large")]
    TooLarge,
    #[error("the other laptop sent a message out of order")]
    Unexpected,
    #[error(transparent)]
    Pairing(#[from] PairingError),
    #[error("network error: {0}")]
    Io(io::Error),
}

impl From<io::Error> for LinkError {
    fn from(e: io::Error) -> Self {
        match e.kind() {
            io::ErrorKind::UnexpectedEof
            | io::ErrorKind::ConnectionReset
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::BrokenPipe => Self::Closed,
            _ => Self::Io(e),
        }
    }
}

/// The old laptop's listener. Serves one device at a time.
pub struct Host {
    listener: TcpListener,
    config: LinkConfig,
}

impl Host {
    /// Starts listening on `addr` (use port 0 for any free port).
    pub async fn bind(addr: SocketAddr, config: LinkConfig) -> io::Result<Self> {
        Ok(Self {
            listener: TcpListener::bind(addr).await?,
            config,
        })
    }

    /// The address and port to announce.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    /// Serves devices one at a time until one completes the handshake and the person can pick
    /// the number. Devices that fail are dropped and the next one is served. Returns an error
    /// only when pairing is locked or the listener fails.
    pub async fn next_peer(
        &self,
        sender: &Mutex<RotatingSender>,
    ) -> Result<HostPending, LinkError> {
        loop {
            wait_until_open(sender).await?;
            let (stream, peer) = self.listener.accept().await?;
            let attempt = serve(stream, sender, self.config);
            tokio::pin!(attempt);
            let outcome = loop {
                tokio::select! {
                    done = &mut attempt => break done,
                    extra = self.listener.accept() => {
                        if let Ok((other, _)) = extra {
                            tokio::spawn(refuse(other));
                        }
                    }
                }
            };
            match outcome {
                Ok((choosing, stream)) => {
                    return Ok(HostPending {
                        choosing,
                        stream,
                        peer,
                    });
                }
                Err(LinkError::Pairing(PairingError::Locked)) => {
                    return Err(LinkError::Pairing(PairingError::Locked));
                }
                // This device failed or went quiet; serve the next one.
                Err(_) => continue,
            }
        }
    }
}

/// The old laptop, offering three numbers to the person.
pub struct HostPending {
    choosing: SenderChoosing,
    stream: TcpStream,
    peer: SocketAddr,
}

impl HostPending {
    /// The three numbers to show, in display order.
    pub fn choices(&self) -> [u8; 3] {
        self.choosing.choices()
    }

    /// When the old laptop stops waiting for the person's pick.
    pub fn deadline(&self) -> Instant {
        self.choosing.deadline()
    }

    /// The network address of the device being paired, for "more details".
    pub fn peer_addr(&self) -> SocketAddr {
        self.peer
    }

    /// Records the person's pick at time `now`. The right number unlocks the old laptop and sends
    /// the approval; anything else ends the pairing.
    pub async fn choose(mut self, picked: u8, now: Instant) -> Result<Link, LinkError> {
        let (paired, approval) = self.choosing.choose(picked, now)?;
        write_frame(&mut self.stream, KIND_PAIRING, &approval).await?;
        Ok(Link {
            paired,
            stream: self.stream,
        })
    }
}

/// Connects the new laptop to the old laptop at `addr` with the code the person typed, and runs
/// the handshake. Returns the number to show while the person picks on the old laptop.
pub async fn connect(
    addr: SocketAddr,
    code: &PairingCode,
    config: LinkConfig,
) -> Result<GuestPending, LinkError> {
    let mut stream = tokio::time::timeout(config.step_timeout, TcpStream::connect(addr))
        .await
        .map_err(|_| LinkError::Timeout)??;
    let msg1 = read_pairing(&mut stream, config.step_timeout).await?;
    let (receiver, msg2) = ReceiverSession::respond(code, &msg1, Instant::now())?;
    write_frame(&mut stream, KIND_PAIRING, &msg2).await?;
    let msg3 = read_pairing(&mut stream, config.step_timeout).await?;
    let (waiting, reveal) = receiver
        .receive(&msg3, Instant::now())
        .map_err(PairingError::from)?;
    write_frame(&mut stream, KIND_PAIRING, &reveal).await?;
    Ok(GuestPending { waiting, stream })
}

/// The new laptop, showing its number and waiting for the person's pick on the old laptop.
pub struct GuestPending {
    waiting: ReceiverAwaitingApproval,
    stream: TcpStream,
}

impl GuestPending {
    /// The number to show on the new laptop.
    pub fn match_number(&self) -> u8 {
        self.waiting.match_number()
    }

    /// When the new laptop stops waiting.
    pub fn deadline(&self) -> Instant {
        self.waiting.deadline()
    }

    /// Waits until the deadline for the old laptop's approval.
    pub async fn approval(mut self) -> Result<Link, LinkError> {
        let wait = self
            .waiting
            .deadline()
            .saturating_duration_since(Instant::now());
        let approval = read_pairing(&mut self.stream, wait).await?;
        let paired = self
            .waiting
            .receive_approval(&approval, Instant::now())
            .map_err(PairingError::from)?;
        Ok(Link {
            paired,
            stream: self.stream,
        })
    }
}

/// A paired, encrypted link. Every message is sealed; anything altered, replayed or forged is
/// rejected.
pub struct Link {
    paired: Paired,
    stream: TcpStream,
}

impl Link {
    /// Seals and sends one message.
    pub async fn send(&mut self, data: &[u8]) -> Result<(), LinkError> {
        let sealed = self.paired.transport_mut().seal(data)?;
        write_frame(&mut self.stream, KIND_DATA, &sealed).await
    }

    /// Receives and opens one message. Waits as long as it takes; wrap in a timeout if needed.
    pub async fn recv(&mut self) -> Result<Zeroizing<Vec<u8>>, LinkError> {
        let (kind, body) = read_frame(&mut self.stream, MAX_DATA_LEN).await?;
        if kind != KIND_DATA {
            return Err(LinkError::Unexpected);
        }
        Ok(self.paired.transport_mut().open(&body)?)
    }
}

macro_rules! redacted_debug {
    ($($t:ty),*) => {$(
        impl fmt::Debug for $t {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(concat!(stringify!($t), "(..)"))
            }
        }
    )*};
}
redacted_debug!(Host, HostPending, GuestPending, Link);

fn lock(sender: &Mutex<RotatingSender>) -> MutexGuard<'_, RotatingSender> {
    // Every update to the sender completes before its lock is released, so its state is
    // consistent even if another thread panicked while holding it.
    sender.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Waits out a pause after a failed guess; refuses to serve once pairing is locked.
async fn wait_until_open(sender: &Mutex<RotatingSender>) -> Result<(), LinkError> {
    loop {
        let status = {
            let mut s = lock(sender);
            s.tick(Instant::now())?;
            s.status()
        };
        match status {
            SenderStatus::Locked => return Err(PairingError::Locked.into()),
            SenderStatus::CoolingDown { until } => {
                tokio::time::sleep(until.saturating_duration_since(Instant::now())).await;
            }
            SenderStatus::Showing { .. } => return Ok(()),
        }
    }
}

async fn serve(
    mut stream: TcpStream,
    sender: &Mutex<RotatingSender>,
    config: LinkConfig,
) -> Result<(SenderChoosing, TcpStream), LinkError> {
    let msg1 = {
        let mut s = lock(sender);
        s.tick(Instant::now())?;
        s.message_1().ok_or(PairingError::CoolingDown)?.to_vec()
    };
    write_frame(&mut stream, KIND_PAIRING, &msg1).await?;
    let msg2 = read_pairing(&mut stream, config.step_timeout).await?;
    let (waiting, msg3) = lock(sender).receive(&msg2, Instant::now())?;
    write_frame(&mut stream, KIND_PAIRING, &msg3).await?;
    let reveal = read_pairing(&mut stream, config.step_timeout).await?;
    let choosing = waiting
        .receive_reveal(&reveal, Instant::now())
        .map_err(PairingError::from)?;
    Ok((choosing, stream))
}

/// Tells a device the old laptop is busy, without reading anything it sent.
async fn refuse(mut stream: TcpStream) {
    let _ = tokio::time::timeout(REFUSE_LINGER, async {
        write_frame(&mut stream, KIND_BUSY, &[]).await?;
        stream.shutdown().await?;
        // Let the device read the notice before the socket closes; its bytes are discarded.
        let mut sink = [0u8; 512];
        while stream.read(&mut sink).await? > 0 {}
        Ok::<(), LinkError>(())
    })
    .await;
}

async fn write_frame(stream: &mut TcpStream, kind: u8, body: &[u8]) -> Result<(), LinkError> {
    let len = u16::try_from(body.len()).map_err(|_| LinkError::TooLarge)?;
    let mut frame = Vec::with_capacity(HEADER_LEN + body.len());
    frame.push(kind);
    frame.extend_from_slice(&len.to_be_bytes());
    frame.extend_from_slice(body);
    stream.write_all(&frame).await?;
    Ok(())
}

async fn read_frame(stream: &mut TcpStream, max: usize) -> Result<(u8, Vec<u8>), LinkError> {
    let mut header = [0u8; HEADER_LEN];
    stream.read_exact(&mut header).await?;
    let len = usize::from(u16::from_be_bytes([header[1], header[2]]));
    if len > max {
        return Err(LinkError::TooLarge);
    }
    let mut body = vec![0u8; len];
    stream.read_exact(&mut body).await?;
    Ok((header[0], body))
}

async fn read_pairing(stream: &mut TcpStream, wait: Duration) -> Result<Vec<u8>, LinkError> {
    let (kind, body) = tokio::time::timeout(wait, read_frame(stream, MAX_MESSAGE_LEN))
        .await
        .map_err(|_| LinkError::Timeout)??;
    match kind {
        KIND_PAIRING => Ok(body),
        KIND_BUSY => Err(LinkError::Busy),
        _ => Err(LinkError::Unexpected),
    }
}
