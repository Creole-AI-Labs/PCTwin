//! PCTwin link: runs pairing over a real network connection and then carries sealed data
//! (Security Design, Part A).
//!
//! # How it works
//!
//! - The old laptop is the [`Host`]: it listens on a TCP port while pairing and serves one device
//!   at a time, and only devices with local-network addresses ([`is_local_peer`]). Any other
//!   device that connects meanwhile is told the old laptop is busy and is closed without anything
//!   it sent being read, so nobody else can spend or disturb the attempt in progress.
//! - The new laptop calls [`connect`] after the person typed the code. It first says whether the
//!   code ends in an even or odd digit, so the host can answer with that code's message 1 even if
//!   the code was replaced a moment ago (the grace period). The parity is the only thing it
//!   reveals, and costs one bit of the code.
//! - The whole handshake must finish within [`LinkConfig::handshake_timeout`]: the person has
//!   already typed the code, so a real new laptop answers at once. Every attempt that fails is
//!   classified where it fails. Only two outcomes are treated as honest: a wrong code (the person
//!   may have mistyped; it counts against the failure budget instead) and a code that is no longer
//!   live (once per address in ten seconds: a person told so types the new code). Everything else, including silence, hanging up and sending the host a status notice,
//!   refuses that address for [`LinkConfig::silence_penalty`].
//! - Devices are told at once when the old laptop is busy, pausing after a wrong code, locked, or
//!   when their code was wrong or has expired, instead of being left to time out. The host keeps
//!   answering while locked, so the app learns about the lock from [`RotatingSender::status`].
//! - After the person's pick on the old laptop, both sides get a [`Link`] that seals every message.
//!
//! Every frame on the wire is `[kind][length, 2 bytes big-endian][body]`. Pairing frames are
//! limited to [`MAX_MESSAGE_LEN`]; anything larger ends that connection before it is read.
//!
//! # Known limits
//!
//! - Someone on the local network can still stop pairing on purpose: wrong guesses from several
//!   addresses trigger the safety stop (pairing locks until the person chooses Start again), and
//!   each new address can hold the single slot for one handshake limit. This is an accepted
//!   nuisance, not a way in (Security Design, decided 6 October 2026).
//! - "Local network" means private, link-local and loopback addresses. Some VPNs use such
//!   addresses too, so the app should listen only on the chosen Wi-Fi interface's own address.
//! - While the person is choosing the number, the host is not accepting: devices that connect then
//!   wait in the operating system's queue and are served on the next [`Host::next_peer`] call.
//! - [`Link::recv`] waits as long as it takes. Cancelling it closes the link, so the app should
//!   keep one receive running rather than wrapping each one in a timeout.

use std::collections::HashMap;
use std::fmt;
use std::io;
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

/// The keys for extra lanes (see [`Link::take_lane_keys`]).
pub use pctwin_pairing::LaneKeys;
use pctwin_pairing::{
    MAX_MESSAGE_LEN, Paired, PairingCode, PairingError, ReceiverAwaitingApproval, ReceiverSession,
    RotatingSender, SenderChoosing, SenderStatus, Transport,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Semaphore;
use zeroize::Zeroizing;

/// Default longest wait for each step the new laptop waits on, and for each send.
pub const STEP_TIMEOUT: Duration = Duration::from_secs(10);
/// Default longest time one device may hold the old laptop's single pairing slot.
pub const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
/// Default time an address whose attempt failed for any reason other than a wrong or expired
/// code is refused.
pub const SILENCE_PENALTY: Duration = Duration::from_secs(30);
/// Default longest wait to reach one of the old laptop's addresses.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);

const KIND_PAIRING: u8 = 1;
const KIND_BUSY: u8 = 2;
const KIND_DATA: u8 = 3;
const KIND_HELLO: u8 = 4;
const KIND_PAUSED: u8 = 5;
const KIND_EXPIRED: u8 = 6;
const KIND_LOCKED: u8 = 7;
const KIND_WRONG_CODE: u8 = 8;
const KIND_LANE: u8 = 9;
/// A quiet connection is probed after this long, every [`KEEPALIVE_INTERVAL`], up to
/// [`KEEPALIVE_PROBES`] times: a laptop that went away (lid closed, out of Wi-Fi range) is noticed
/// in about half a minute rather than the systems' default of up to two hours. The operating
/// system answers probes itself, so a laptop busy reading a slow drive is never mistaken for gone.
pub const KEEPALIVE_IDLE: Duration = Duration::from_secs(10);
pub const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(5);
pub const KEEPALIVE_PROBES: u32 = 4;
const HEADER_LEN: usize = 3;
const MAX_DATA_LEN: usize = u16::MAX as usize;
const SESSION_TAG: std::ops::Range<usize> = 2..10;
/// How long a device gets to read a notice before it is closed.
const NOTICE_LINGER: Duration = Duration::from_secs(1);
/// Most devices being refused at once; beyond this, extra connections are closed without a notice.
const MAX_REFUSALS: usize = 64;
/// Most addresses remembered as penalised.
const MAX_PENALISED: usize = 1024;
/// One expired-code answer per address in this window is honest; a second costs the penalty.
const EXPIRED_WINDOW: Duration = Duration::from_secs(10);
/// Pause after the operating system refuses to accept a connection (for example, out of handles).
const ACCEPT_BACKOFF: Duration = Duration::from_millis(50);

/// Timing for one link.
#[derive(Debug, Clone, Copy)]
pub struct LinkConfig {
    /// Longest wait for each step the new laptop waits on, and for each send.
    pub step_timeout: Duration,
    /// Longest time one device may hold the old laptop's pairing slot, from hello to reveal.
    pub handshake_timeout: Duration,
    /// How long an address whose attempt failed other than by a wrong or expired code is refused.
    pub silence_penalty: Duration,
    /// Longest wait to reach the old laptop at one address before trying the next.
    pub connect_timeout: Duration,
}

impl Default for LinkConfig {
    fn default() -> Self {
        Self {
            step_timeout: STEP_TIMEOUT,
            handshake_timeout: HANDSHAKE_TIMEOUT,
            silence_penalty: SILENCE_PENALTY,
            connect_timeout: CONNECT_TIMEOUT,
        }
    }
}

/// Why connecting, pairing or the link failed.
#[derive(Debug, thiserror::Error)]
pub enum LinkError {
    #[error("the old laptop is pairing with another device; try again in a moment")]
    Busy,
    #[error("the old laptop could not be reached at that address")]
    Unreachable,
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

/// Whether `ip` has a local-network address: private, link-local or loopback. The host serves
/// only these.
pub fn is_local_peer(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_private() || v4.is_link_local() || v4.is_loopback(),
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => is_local_peer(IpAddr::V4(v4)),
            None => v6.is_loopback() || v6.is_unicast_link_local() || v6.is_unique_local(),
        },
    }
}

/// How a failed attempt is judged.
enum Blame {
    /// A wrong code or a code that is no longer live: no penalty for the address.
    Honest,
    /// Anything else: the address is refused for a while.
    Address,
}

struct Failure {
    error: LinkError,
    blame: Blame,
}

impl<E: Into<LinkError>> From<E> for Failure {
    /// By default a failure is the address's fault; honest outcomes are marked where they happen.
    fn from(e: E) -> Self {
        Self {
            error: e.into(),
            blame: Blame::Address,
        }
    }
}

fn honest(error: impl Into<LinkError>) -> Failure {
    Failure {
        error: error.into(),
        blame: Blame::Honest,
    }
}

/// The old laptop's listener. Serves one device at a time.
pub struct Host {
    listener: TcpListener,
    config: LinkConfig,
    refusals: Arc<Semaphore>,
    penalised: Mutex<HashMap<IpAddr, Instant>>,
    /// When each address was last told its code had expired.
    expired: Mutex<HashMap<IpAddr, Instant>>,
    /// Which peer addresses are served: always [`is_local_peer`], except in this crate's own
    /// tests, which run over loopback and need to see the rule applied.
    allowed: fn(IpAddr) -> bool,
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
            expired: Mutex::new(HashMap::new()),
            allowed: is_local_peer,
        })
    }

    /// The address and port to announce.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    /// Serves devices one at a time until one completes the handshake and the person can pick
    /// the number. Devices that fail are dropped and the next one is served. While pairing is
    /// locked it keeps telling arriving devices so; after the person chooses Start again
    /// ([`RotatingSender::unlock`]) it serves them again. Fails only if the system's secure random
    /// source fails.
    pub async fn next_peer(
        &self,
        sender: &Mutex<RotatingSender>,
    ) -> Result<HostPending, LinkError> {
        loop {
            let Some((stream, peer)) = self.accept_local().await else {
                continue;
            };
            match current_status(sender)? {
                SenderStatus::Locked => {
                    self.refuse(stream, KIND_LOCKED);
                    continue;
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

            let limit = self.config.handshake_timeout;
            let attempt = async {
                tokio::time::timeout(limit, serve(stream, sender, self.config))
                    .await
                    .unwrap_or_else(|_| Err(LinkError::Timeout.into()))
            };
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
                Err(Failure {
                    error: LinkError::Pairing(PairingError::Random),
                    ..
                }) => return Err(PairingError::Random.into()),
                Err(Failure {
                    blame: Blame::Address,
                    ..
                }) => self.penalise(peer.ip()),
                // A person told their code expired types the new one; asking again for a dead
                // code soon after is not honest any more. (Wrong codes are already limited by the
                // pause and the safety stop.)
                Err(Failure {
                    error: LinkError::Pairing(PairingError::Expired),
                    blame: Blame::Honest,
                }) => {
                    if self.expired_again(peer.ip()) {
                        self.penalise(peer.ip());
                    }
                }
                Err(Failure {
                    blame: Blame::Honest,
                    ..
                }) => {}
            }
        }
    }

    /// Records an expired-code answer for `ip`; true if it already had one within
    /// [`EXPIRED_WINDOW`].
    fn expired_again(&self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let mut map = self.expired.lock().unwrap_or_else(PoisonError::into_inner);
        map.retain(|_, at| now.duration_since(*at) < EXPIRED_WINDOW);
        let key = penalty_key(ip);
        let again = map.contains_key(&key);
        if map.len() < MAX_PENALISED {
            map.insert(key, now);
        }
        again
    }

    /// Accepts one connection from the local network. Connections from elsewhere are closed at
    /// once; an accept error (for example, out of handles) pauses briefly and is not fatal.
    async fn accept_local(&self) -> Option<(TcpStream, SocketAddr)> {
        match self.listener.accept().await {
            Ok((stream, peer)) if (self.allowed)(peer.ip()) => {
                keep_alive(&stream);
                Some((stream, peer))
            }
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
        map.get(&penalty_key(ip)).is_some_and(|&until| now < until)
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
            map.insert(penalty_key(ip), now + self.config.silence_penalty);
        }
    }
}

/// IPv6 devices choose their own address within a /64, so penalties apply to the whole /64.
fn penalty_key(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) if v6.to_ipv4_mapped().is_none() => {
            let s = v6.segments();
            IpAddr::V6(Ipv6Addr::new(s[0], s[1], s[2], s[3], 0, 0, 0, 0))
        }
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map_or(IpAddr::V6(v6), IpAddr::V4),
        v4 => v4,
    }
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
        Ok(Link::new(paired, self.stream, self.peer, self.send_timeout))
    }
}

/// Connects the new laptop to the old laptop at `addr` with the code the person typed, and runs
/// the handshake. Returns the number to show while the person picks on the old laptop.
///
/// Fails with [`LinkError::Unreachable`] only if nothing was sent: then trying another address of
/// the same old laptop cannot spend a guess. Any later failure means an attempt was made.
pub async fn connect(
    addr: SocketAddr,
    code: &PairingCode,
    config: LinkConfig,
) -> Result<GuestPending, LinkError> {
    let wait = config.step_timeout;
    let mut stream = tokio::time::timeout(config.connect_timeout, TcpStream::connect(addr))
        .await
        .map_err(|_| LinkError::Unreachable)?
        .map_err(|_| LinkError::Unreachable)?;
    keep_alive(&stream);
    write_frame(&mut stream, KIND_HELLO, &[code.parity()], wait).await?;
    let msg1 = read_from_host(&mut stream, wait).await?;
    let (receiver, msg2) = ReceiverSession::respond(code, &msg1, Instant::now())?;
    write_frame(&mut stream, KIND_PAIRING, &msg2, wait).await?;
    let msg3 = read_from_host(&mut stream, wait).await?;
    let (waiting, reveal) = receiver
        .receive(&msg3, Instant::now())
        .map_err(PairingError::from)?;
    write_frame(&mut stream, KIND_PAIRING, &reveal, wait).await?;
    Ok(GuestPending {
        waiting,
        stream,
        host: addr,
        send_timeout: wait,
    })
}

/// The new laptop, showing its number and waiting for the person's pick on the old laptop.
pub struct GuestPending {
    waiting: ReceiverAwaitingApproval,
    stream: TcpStream,
    host: SocketAddr,
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
        let approval = read_from_host(&mut self.stream, wait).await?;
        let paired = self
            .waiting
            .receive_approval(&approval, Instant::now())
            .map_err(PairingError::from)?;
        Ok(Link::new(paired, self.stream, self.host, self.send_timeout))
    }
}

/// A paired, encrypted link. Every message is sealed; anything altered, replayed or forged is
/// rejected. A half-sent or half-read frame would leave the stream out of step, so the link is
/// closed for good after any failure, and also if a [`send`](Self::send) or [`recv`](Self::recv)
/// is cancelled before it finishes (for example by a timeout around it).
pub struct Link {
    transport: Transport,
    /// On the main link only, until the app takes them: the keys for extra lanes.
    lanes: Option<LaneKeys>,
    stream: TcpStream,
    peer: SocketAddr,
    send_timeout: Duration,
    broken: bool,
}

impl Link {
    fn new(paired: Paired, stream: TcpStream, peer: SocketAddr, send_timeout: Duration) -> Self {
        let (transport, lanes) = paired.into_parts();
        Self {
            transport,
            lanes: Some(lanes),
            stream,
            peer,
            send_timeout,
            broken: false,
        }
    }

    /// The other laptop's address: where the new laptop opens lanes, and the only address the old
    /// laptop accepts them from.
    pub fn peer_addr(&self) -> SocketAddr {
        self.peer
    }

    /// Whether the operating system is probing this connection while it is quiet (see
    /// [`KEEPALIVE_IDLE`]), so a laptop that went away is noticed.
    pub fn keeps_alive(&self) -> bool {
        socket2::SockRef::from(&self.stream)
            .keepalive()
            .unwrap_or(false)
    }

    /// The keys for extra lanes, handed out once (from the main link only), so the app can open
    /// or accept lanes while this link carries the move.
    pub fn take_lane_keys(&mut self) -> Option<LaneKeys> {
        self.lanes.take()
    }

    /// Seals and sends one message. Fails with [`LinkError::Timeout`] if the other laptop stops
    /// reading. A message too large to seal is refused without harming the link.
    pub async fn send(&mut self, data: &[u8]) -> Result<(), LinkError> {
        if self.broken {
            return Err(LinkError::Closed);
        }
        let sealed = self.transport.seal(data)?;
        // Marked broken until the frame is fully written, so a cancelled send cannot be reused.
        self.broken = true;
        let sent = write_frame(&mut self.stream, KIND_DATA, &sealed, self.send_timeout).await;
        self.broken = sent.is_err();
        sent
    }

    /// Receives and opens one message. Waits as long as it takes. Cancelling it (for example
    /// with a timeout) closes the link, since part of a frame may already have been read.
    pub async fn recv(&mut self) -> Result<Zeroizing<Vec<u8>>, LinkError> {
        if self.broken {
            return Err(LinkError::Closed);
        }
        // Marked broken until a whole frame is read and opened, so a cancelled read cannot be reused.
        self.broken = true;
        let opened = async {
            let (kind, body) = read_frame(&mut self.stream, MAX_DATA_LEN).await?;
            if kind != KIND_DATA {
                return Err(LinkError::Unexpected);
            }
            Ok(self.transport.open(&body)?)
        }
        .await;
        self.broken = opened.is_err();
        opened
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
redacted_debug!(Host, HostPending, GuestPending, Link, LaneListener);

/// New laptop: opens an extra lane to the old laptop at `addr` (the main link's
/// [`Link::peer_addr`]), with no new code: the lane's own short handshake proves both laptops hold
/// this pairing's keys. Each try uses the next lane number, even if it fails; a refusal means
/// stop opening lanes and carry on over the ones already open.
pub async fn open_lane(
    addr: SocketAddr,
    keys: &mut LaneKeys,
    config: LinkConfig,
) -> Result<Link, LinkError> {
    let wait = config.step_timeout;
    let mut stream = tokio::time::timeout(config.connect_timeout, TcpStream::connect(addr))
        .await
        .map_err(|_| LinkError::Unreachable)?
        .map_err(|_| LinkError::Unreachable)?;
    keep_alive(&stream);
    let (opening, msg1) = keys.open_lane(Instant::now())?;
    write_frame(&mut stream, KIND_LANE, &msg1, wait).await?;
    let msg2 = match read_timed(&mut stream, wait).await? {
        (KIND_LANE, body) => body,
        (KIND_BUSY, _) => return Err(LinkError::Busy),
        _ => return Err(LinkError::Unexpected),
    };
    let (transport, msg3) = opening.receive(&msg2, Instant::now())?;
    write_frame(&mut stream, KIND_LANE, &msg3, wait).await?;
    Ok(Link {
        transport,
        lanes: None,
        stream,
        peer: addr,
        send_timeout: wait,
        broken: false,
    })
}

/// Old laptop, during the move: accepts extra lanes from the paired new laptop, one handshake at a
/// time. Connections from any other address are closed without a word; a device trying to pair is
/// told the old laptop is busy; a connection that sends anything but a genuine lane, or nothing
/// within the step limit, is dropped and the next one served.
pub struct LaneListener {
    host: Host,
    keys: LaneKeys,
    peer: IpAddr,
}

impl Host {
    /// After pairing: serve extra lanes for the move, from the paired laptop at `peer` only.
    pub fn into_lanes(self, keys: LaneKeys, peer: SocketAddr) -> LaneListener {
        LaneListener {
            host: self,
            keys,
            peer: penalty_key(peer.ip()),
        }
    }
}

impl LaneListener {
    /// Waits for the next genuine lane and returns it.
    pub async fn accept(&mut self) -> Result<Link, LinkError> {
        loop {
            let Some((mut stream, from)) = self.host.accept_local().await else {
                continue;
            };
            if penalty_key(from.ip()) != self.peer {
                // Not the paired laptop: nothing is read or said.
                continue;
            }
            let wait = self.host.config.step_timeout;
            let keys = &mut self.keys;
            let attempt = async {
                let msg1 = match read_timed(&mut stream, wait).await? {
                    (KIND_LANE, body) => body,
                    (KIND_HELLO, _) => {
                        notify(&mut stream, KIND_BUSY).await;
                        return Err(LinkError::Busy);
                    }
                    _ => return Err(LinkError::Unexpected),
                };
                let (accepting, msg2) = keys.accept_lane(&msg1, Instant::now())?;
                write_frame(&mut stream, KIND_LANE, &msg2, wait).await?;
                let msg3 = match read_timed(&mut stream, wait).await? {
                    (KIND_LANE, body) => body,
                    _ => return Err(LinkError::Unexpected),
                };
                Ok(accepting.confirm(&msg3, Instant::now())?)
            };
            if let Ok(transport) = attempt.await {
                return Ok(Link {
                    transport,
                    lanes: None,
                    stream,
                    peer: from,
                    send_timeout: wait,
                    broken: false,
                });
            }
        }
    }
}

/// Turns on the operating system's probing of a quiet connection. If a system refuses the
/// settings, the connection still works; a lost laptop is then noticed only by its own timeouts.
fn keep_alive(stream: &TcpStream) {
    let probing = socket2::TcpKeepalive::new()
        .with_time(KEEPALIVE_IDLE)
        .with_interval(KEEPALIVE_INTERVAL)
        .with_retries(KEEPALIVE_PROBES);
    let _ = socket2::SockRef::from(stream).set_tcp_keepalive(&probing);
}

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

/// Runs one device's attempt. Every failure is the address's fault unless marked honest.
async fn serve(
    mut stream: TcpStream,
    sender: &Mutex<RotatingSender>,
    config: LinkConfig,
) -> Result<(SenderChoosing, TcpStream), Failure> {
    // Each read is bounded by the whole-handshake limit around this function.
    let wait = config.handshake_timeout;
    let hello = read_from_guest(&mut stream, KIND_HELLO, wait).await?;
    let parity = match hello.as_slice() {
        [p @ (0 | 1)] => *p,
        _ => return Err(LinkError::Unexpected.into()),
    };
    let msg1 = {
        let mut s = lock(sender);
        s.tick(Instant::now())?;
        s.message_1_for(parity).map(<[u8]>::to_vec)
    };
    let Some(msg1) = msg1 else {
        notify(&mut stream, KIND_EXPIRED).await;
        return Err(honest(PairingError::Expired));
    };
    write_frame(&mut stream, KIND_PAIRING, &msg1, wait).await?;

    let msg2 = read_from_guest(&mut stream, KIND_PAIRING, wait).await?;
    if msg2.get(SESSION_TAG) != msg1.get(SESSION_TAG) {
        // A reply must answer the message 1 this device was given.
        return Err(LinkError::Unexpected.into());
    }
    let received = lock(sender).receive(&msg2, Instant::now());
    let (waiting, msg3) = match received {
        Ok(done) => done,
        // The code this device holds ran out of grace during the handshake.
        Err(PairingError::UnknownSession) => {
            notify(&mut stream, KIND_EXPIRED).await;
            return Err(honest(PairingError::Expired));
        }
        // A wrong code: counted against the failure budget, which may now have locked pairing.
        Err(PairingError::HandshakeFailed) => {
            let locked = current_status(sender)? == SenderStatus::Locked;
            let (kind, error) = if locked {
                (KIND_LOCKED, PairingError::Locked)
            } else {
                (KIND_WRONG_CODE, PairingError::HandshakeFailed)
            };
            notify(&mut stream, kind).await;
            return Err(honest(error));
        }
        Err(e) => return Err(e.into()),
    };
    write_frame(&mut stream, KIND_PAIRING, &msg3, wait).await?;
    let reveal = read_from_guest(&mut stream, KIND_PAIRING, wait).await?;
    let choosing = waiting
        .receive_reveal(&reveal, Instant::now())
        .map_err(PairingError::from)?;
    Ok((choosing, stream))
}

/// Sends a notice on a connection that is about to close, without waiting long.
async fn notify(stream: &mut TcpStream, kind: u8) {
    let _ = write_frame(stream, kind, &[], NOTICE_LINGER).await;
    let _ = tokio::time::timeout(NOTICE_LINGER, stream.shutdown()).await;
}

/// Sends a notice of `kind` and closes, without reading anything the device sent.
async fn send_notice_and_close(mut stream: TcpStream, kind: u8) {
    let _ = tokio::time::timeout(NOTICE_LINGER, async {
        write_frame(&mut stream, kind, &[], NOTICE_LINGER).await?;
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

async fn read_timed(stream: &mut TcpStream, wait: Duration) -> Result<(u8, Vec<u8>), LinkError> {
    tokio::time::timeout(wait, read_frame(stream, MAX_MESSAGE_LEN))
        .await
        .map_err(|_| LinkError::Timeout)?
}

/// The host reads exactly the kind it expects. Notices only ever travel from host to guest, so a
/// guest that sends one is breaking the protocol.
async fn read_from_guest(
    stream: &mut TcpStream,
    expected: u8,
    wait: Duration,
) -> Result<Vec<u8>, LinkError> {
    match read_timed(stream, wait).await? {
        (kind, body) if kind == expected => Ok(body),
        _ => Err(LinkError::Unexpected),
    }
}

/// The guest reads a pairing message, or a notice explaining why the host stopped.
async fn read_from_host(stream: &mut TcpStream, wait: Duration) -> Result<Vec<u8>, LinkError> {
    let (kind, body) = read_timed(stream, wait).await?;
    match kind {
        KIND_PAIRING => Ok(body),
        KIND_BUSY => Err(LinkError::Busy),
        KIND_PAUSED => Err(PairingError::CoolingDown.into()),
        KIND_EXPIRED => Err(PairingError::Expired.into()),
        KIND_LOCKED => Err(PairingError::Locked.into()),
        KIND_WRONG_CODE => Err(PairingError::HandshakeFailed.into()),
        _ => Err(LinkError::Unexpected),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(s: &str) -> IpAddr {
        penalty_key(s.parse().unwrap())
    }

    #[test]
    fn ipv6_penalties_cover_the_whole_64_and_nothing_wider() {
        // A device can pick any address inside its /64, so the penalty must cover all of them.
        assert_eq!(key("fd00:1:2:3::1"), key("fd00:1:2:3:ffff:ffff:ffff:fffe"));
        assert_eq!(key("fe80::1"), key("fe80::abcd:1234"));
        // A neighbouring /64 is a different network and is not penalised with it.
        assert_ne!(key("fd00:1:2:3::1"), key("fd00:1:2:4::1"));
        assert_eq!(
            key("fd00:1:2:3::1"),
            "fd00:1:2:3::".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn ipv4_penalties_are_per_address_including_mapped_forms() {
        assert_eq!(key("192.168.1.7"), "192.168.1.7".parse::<IpAddr>().unwrap());
        assert_ne!(key("192.168.1.7"), key("192.168.1.8"));
        // The same IPv4 device seen through an IPv6 socket is the same address.
        assert_eq!(key("::ffff:192.168.1.7"), key("192.168.1.7"));
    }

    #[tokio::test]
    async fn devices_the_rule_does_not_allow_are_closed_without_a_word() {
        let mut host = Host::bind("127.0.0.1:0".parse().unwrap(), LinkConfig::default())
            .await
            .unwrap();
        // Loopback stands in for an address outside the local network.
        host.allowed = |_| false;
        let addr = host.local_addr().unwrap();
        let sender = Mutex::new(RotatingSender::new(Instant::now()).unwrap());
        let serving = async { host.next_peer(&sender).await };
        let outsider = async {
            let mut raw = TcpStream::connect(addr).await.unwrap();
            let _ = raw.write_all(&[KIND_HELLO, 0, 1, 0]).await;
            let mut reply = Vec::new();
            let _ = tokio::time::timeout(Duration::from_secs(2), raw.read_to_end(&mut reply)).await;
            reply
        };
        tokio::select! {
            _ = serving => panic!("an outsider must never be served"),
            reply = outsider => assert!(reply.is_empty(), "no notice, nothing at all"),
        }
    }
}
