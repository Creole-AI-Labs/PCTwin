//! PCTwin link: runs pairing over a real network connection and then carries sealed data
//! (Security Design, Part A).
//!
//! # How it works
//!
//! - The old laptop is the [`Host`]: it listens on a TCP port while pairing and serves one device
//!   at a time, and only devices on the local network ([`is_local_peer`]). Any other device that
//!   connects meanwhile is told the old laptop is busy and is closed without anything it sent
//!   being read, so nobody else can spend or disturb the attempt in progress.
//! - The new laptop calls [`connect`] after the person typed the code. It first says whether the
//!   code ends in an even or odd digit, so the host can answer with that code's message 1 even if
//!   the code was replaced a moment ago (the grace period). The parity is the only thing it
//!   reveals, and costs one bit of the code.
//! - The first reply must come within [`LinkConfig::first_step_timeout`]; later steps within
//!   [`LinkConfig::step_timeout`]. A device that goes quiet or breaks the protocol is dropped,
//!   does not count as a guess, and its address is refused for [`LinkConfig::silence_penalty`].
//!   A wrong code counts against the old laptop's failure budget.
//! - A device that arrives while the old laptop is pausing after a wrong code, or whose code is no
//!   longer live, is told so at once instead of being left to time out.
//! - After the person's pick on the old laptop, both sides get a [`Link`] that seals every message.
//!
//! Every frame on the wire is `[kind][length, 2 bytes big-endian][body]`. Pairing frames are
//! limited to [`MAX_MESSAGE_LEN`]; anything larger ends that connection before it is read.
//!
//! # Known limits
//!
//! - Someone on the local network can still stop pairing on purpose: wrong guesses from several
//!   addresses trigger the safety stop (pairing locks until the person chooses Start again), and
//!   a new address can hold the single slot for one first-step wait. This is an accepted nuisance,
//!   not a way in (Security Design, decided 6 October 2026).
//! - While the person is choosing the number, the host is not accepting: devices that connect then
//!   wait in the operating system's queue and are served on the next [`Host::next_peer`] call.
//! - [`Link::recv`] waits as long as it takes; the app decides how long a quiet link may last.

use std::collections::HashMap;
use std::fmt;
use std::io;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use pctwin_pairing::{
    MAX_MESSAGE_LEN, Paired, PairingCode, PairingError, ReceiverAwaitingApproval, ReceiverSession,
    RotatingSender, SenderChoosing, SenderStatus,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Semaphore;
use zeroize::Zeroizing;

/// Default longest wait for each later pairing step from the other laptop.
pub const STEP_TIMEOUT: Duration = Duration::from_secs(10);
/// Default longest wait for the new laptop's first message: it has already typed the code.
pub const FIRST_STEP_TIMEOUT: Duration = Duration::from_secs(3);
/// Default time an address that held the slot in silence, or broke the protocol, is refused.
pub const SILENCE_PENALTY: Duration = Duration::from_secs(30);

const KIND_PAIRING: u8 = 1;
const KIND_BUSY: u8 = 2;
const KIND_DATA: u8 = 3;
const KIND_HELLO: u8 = 4;
const KIND_PAUSED: u8 = 5;
const KIND_EXPIRED: u8 = 6;
const KIND_LOCKED: u8 = 7;
const HEADER_LEN: usize = 3;
const MAX_DATA_LEN: usize = u16::MAX as usize;
/// How long a refused device gets to read its notice before it is closed.
const REFUSE_LINGER: Duration = Duration::from_secs(1);
/// Most devices being refused at once; beyond this, extra connections are closed without a notice.
const MAX_REFUSALS: usize = 64;
/// Most addresses remembered as penalised.
const MAX_PENALISED: usize = 1024;
/// Pause after the operating system refuses to accept a connection (for example, out of handles).
const ACCEPT_BACKOFF: Duration = Duration::from_millis(50);

/// Timing for one link.
#[derive(Debug, Clone, Copy)]
pub struct LinkConfig {
    /// Longest wait for each later pairing step, and for each sealed message to be sent.
    pub step_timeout: Duration,
    /// Longest wait for the new laptop's first message.
    pub first_step_timeout: Duration,
    /// How long an address that held the slot in silence, or broke the protocol, is refused.
    pub silence_penalty: Duration,
}

impl Default for LinkConfig {
    fn default() -> Self {
        Self {
            step_timeout: STEP_TIMEOUT,
            first_step_timeout: FIRST_STEP_TIMEOUT,
            silence_penalty: SILENCE_PENALTY,
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

/// Whether `ip` is on the local network: private, link-local or loopback. The host serves only
/// these, so a laptop that is also on a VPN or the open internet is not reachable from there.
pub fn is_local_peer(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_private() || v4.is_link_local() || v4.is_loopback(),
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => is_local_peer(IpAddr::V4(v4)),
            None => v6.is_loopback() || v6.is_unicast_link_local() || v6.is_unique_local(),
        },
    }
}

/// The old laptop's listener. Serves one device at a time.
pub struct Host {
    listener: TcpListener,
    config: LinkConfig,
    refusals: Arc<Semaphore>,
    penalised: Mutex<HashMap<IpAddr, Instant>>,
}

impl Host {
    /// Starts listening on `addr` (use port 0 for any free port). Bind the local network
    /// interface's own address, not every interface.
    pub async fn bind(addr: SocketAddr, config: LinkConfig) -> io::Result<Self> {
        Ok(Self {
            listener: TcpListener::bind(addr).await?,
            config,
            refusals: Arc::new(Semaphore::new(MAX_REFUSALS)),
            penalised: Mutex::new(HashMap::new()),
        })
    }

    /// The address and port to announce.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    /// Serves devices one at a time until one completes the handshake and the person can pick
    /// the number. Devices that fail are dropped and the next one is served. Returns an error
    /// only when pairing is locked; the app then offers Start again ([`RotatingSender::unlock`]).
    pub async fn next_peer(
        &self,
        sender: &Mutex<RotatingSender>,
    ) -> Result<HostPending, LinkError> {
        loop {
            if current_status(sender)? == SenderStatus::Locked {
                return Err(PairingError::Locked.into());
            }
            let Some((stream, peer)) = self.accept_local().await else {
                continue;
            };
            match current_status(sender)? {
                SenderStatus::Locked => {
                    self.refuse(stream, KIND_LOCKED);
                    return Err(PairingError::Locked.into());
                }
                SenderStatus::CoolingDown { .. } => {
                    self.refuse(stream, KIND_PAUSED);
                    continue;
                }
                SenderStatus::Showing { .. } => {}
            }
            if self.is_penalised(peer.ip()) {
                self.refuse(stream, KIND_BUSY);
                continue;
            }

            let attempt = serve(stream, sender, self.config);
            tokio::pin!(attempt);
            let outcome = loop {
                tokio::select! {
                    done = &mut attempt => break done,
                    extra = self.accept_local() => {
                        if let Some((other, _)) = extra {
                            self.refuse(other, KIND_BUSY);
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
                        send_timeout: self.config.step_timeout,
                    });
                }
                Err(e) => {
                    if deserves_penalty(&e) {
                        self.penalise(peer.ip());
                    }
                    // Serve the next device; a lock is reported at the top of the loop.
                }
            }
        }
    }

    /// Accepts one connection from the local network. Connections from elsewhere are closed at
    /// once; an accept error (for example, out of handles) pauses briefly and is not fatal.
    async fn accept_local(&self) -> Option<(TcpStream, SocketAddr)> {
        match self.listener.accept().await {
            Ok((stream, peer)) if is_local_peer(peer.ip()) => Some((stream, peer)),
            Ok(_) => None,
            Err(_) => {
                tokio::time::sleep(ACCEPT_BACKOFF).await;
                None
            }
        }
    }

    /// Sends a short notice and closes, without reading anything the device sent. Only a bounded
    /// number of refusals run at once; past that, the connection is simply closed.
    fn refuse(&self, stream: TcpStream, kind: u8) {
        if let Ok(permit) = self.refusals.clone().try_acquire_owned() {
            tokio::spawn(async move {
                send_notice_and_close(stream, kind).await;
                drop(permit);
            });
        }
    }

    fn is_penalised(&self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let map = self
            .penalised
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        map.get(&ip).is_some_and(|&until| now < until)
    }

    fn penalise(&self, ip: IpAddr) {
        if self.config.silence_penalty.is_zero() {
            return;
        }
        let now = Instant::now();
        let mut map = self
            .penalised
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        map.retain(|_, until| now < *until);
        if map.len() < MAX_PENALISED {
            map.insert(ip, now + self.config.silence_penalty);
        }
    }
}

/// Silence and protocol breakage cost the address a penalty. A plausible wrong code does not:
/// the person may simply have mistyped, and the failure budget already counts it.
fn deserves_penalty(e: &LinkError) -> bool {
    matches!(
        e,
        LinkError::Timeout
            | LinkError::TooLarge
            | LinkError::Unexpected
            | LinkError::Pairing(
                PairingError::Malformed
                    | PairingError::InvalidKey
                    | PairingError::UnknownSession
                    | PairingError::UnsupportedVersion(_)
            )
    )
}

/// The old laptop, offering three numbers to the person.
pub struct HostPending {
    choosing: SenderChoosing,
    stream: TcpStream,
    peer: SocketAddr,
    send_timeout: Duration,
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
        write_frame(&mut self.stream, KIND_PAIRING, &approval, self.send_timeout).await?;
        Ok(Link {
            paired,
            stream: self.stream,
            send_timeout: self.send_timeout,
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
    let wait = config.step_timeout;
    let mut stream = tokio::time::timeout(wait, TcpStream::connect(addr))
        .await
        .map_err(|_| LinkError::Timeout)??;
    write_frame(&mut stream, KIND_HELLO, &[code.parity()], wait).await?;
    let msg1 = read_pairing(&mut stream, wait).await?;
    let (receiver, msg2) = ReceiverSession::respond(code, &msg1, Instant::now())?;
    write_frame(&mut stream, KIND_PAIRING, &msg2, wait).await?;
    let msg3 = read_pairing(&mut stream, wait).await?;
    let (waiting, reveal) = receiver
        .receive(&msg3, Instant::now())
        .map_err(PairingError::from)?;
    write_frame(&mut stream, KIND_PAIRING, &reveal, wait).await?;
    Ok(GuestPending {
        waiting,
        stream,
        send_timeout: wait,
    })
}

/// The new laptop, showing its number and waiting for the person's pick on the old laptop.
pub struct GuestPending {
    waiting: ReceiverAwaitingApproval,
    stream: TcpStream,
    send_timeout: Duration,
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
            send_timeout: self.send_timeout,
        })
    }
}

/// A paired, encrypted link. Every message is sealed; anything altered, replayed or forged is
/// rejected.
pub struct Link {
    paired: Paired,
    stream: TcpStream,
    send_timeout: Duration,
}

impl Link {
    /// Seals and sends one message. Fails with [`LinkError::Timeout`] if the other laptop stops
    /// reading.
    pub async fn send(&mut self, data: &[u8]) -> Result<(), LinkError> {
        let sealed = self.paired.transport_mut().seal(data)?;
        write_frame(&mut self.stream, KIND_DATA, &sealed, self.send_timeout).await
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

fn current_status(sender: &Mutex<RotatingSender>) -> Result<SenderStatus, LinkError> {
    let mut s = lock(sender);
    s.tick(Instant::now())?;
    Ok(s.status())
}

async fn serve(
    mut stream: TcpStream,
    sender: &Mutex<RotatingSender>,
    config: LinkConfig,
) -> Result<(SenderChoosing, TcpStream), LinkError> {
    let wait = config.step_timeout;
    let hello = read_kind(&mut stream, KIND_HELLO, config.first_step_timeout).await?;
    let parity = match hello.as_slice() {
        [p @ (0 | 1)] => *p,
        _ => return Err(LinkError::Unexpected),
    };
    let msg1 = {
        let mut s = lock(sender);
        s.tick(Instant::now())?;
        s.message_1_for(parity).map(<[u8]>::to_vec)
    };
    let Some(msg1) = msg1 else {
        write_frame(&mut stream, KIND_EXPIRED, &[], wait).await?;
        return Err(PairingError::Expired.into());
    };
    write_frame(&mut stream, KIND_PAIRING, &msg1, wait).await?;
    let msg2 = read_pairing(&mut stream, wait).await?;
    let (waiting, msg3) = lock(sender).receive(&msg2, Instant::now())?;
    write_frame(&mut stream, KIND_PAIRING, &msg3, wait).await?;
    let reveal = read_pairing(&mut stream, wait).await?;
    let choosing = waiting
        .receive_reveal(&reveal, Instant::now())
        .map_err(PairingError::from)?;
    Ok((choosing, stream))
}

/// Sends a notice of `kind` and closes, without reading anything the device sent.
async fn send_notice_and_close(mut stream: TcpStream, kind: u8) {
    let _ = tokio::time::timeout(REFUSE_LINGER, async {
        write_frame(&mut stream, kind, &[], REFUSE_LINGER).await?;
        stream.shutdown().await?;
        // Let the device read the notice before the socket closes; its bytes are discarded.
        let mut sink = [0u8; 512];
        while stream.read(&mut sink).await? > 0 {}
        Ok::<(), LinkError>(())
    })
    .await;
}

async fn write_frame(
    stream: &mut TcpStream,
    kind: u8,
    body: &[u8],
    wait: Duration,
) -> Result<(), LinkError> {
    let len = u16::try_from(body.len()).map_err(|_| LinkError::TooLarge)?;
    let mut frame = Vec::with_capacity(HEADER_LEN + body.len());
    frame.push(kind);
    frame.extend_from_slice(&len.to_be_bytes());
    frame.extend_from_slice(body);
    tokio::time::timeout(wait, stream.write_all(&frame))
        .await
        .map_err(|_| LinkError::Timeout)??;
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

async fn read_kind(
    stream: &mut TcpStream,
    expected: u8,
    wait: Duration,
) -> Result<Vec<u8>, LinkError> {
    let (kind, body) = tokio::time::timeout(wait, read_frame(stream, MAX_MESSAGE_LEN))
        .await
        .map_err(|_| LinkError::Timeout)??;
    match kind {
        k if k == expected => Ok(body),
        KIND_BUSY => Err(LinkError::Busy),
        KIND_PAUSED => Err(PairingError::CoolingDown.into()),
        KIND_EXPIRED => Err(PairingError::Expired.into()),
        KIND_LOCKED => Err(PairingError::Locked.into()),
        _ => Err(LinkError::Unexpected),
    }
}

async fn read_pairing(stream: &mut TcpStream, wait: Duration) -> Result<Vec<u8>, LinkError> {
    read_kind(stream, KIND_PAIRING, wait).await
}
