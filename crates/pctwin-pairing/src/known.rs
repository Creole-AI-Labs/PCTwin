//! Remembered connections (Security Design Part T): after the first sealed pairing each laptop
//! makes a long-term key pair for that connection ([`StaticKeys`]), and the two swap only the
//! public halves inside the sealed link. Later sessions need no code: they use the Noise `KK`
//! pattern, where each side already knows the other's static public key, with fresh ephemeral
//! keys every time (forward secrecy for the session's traffic).
//!
//! # The three messages
//!
//! 1. The laptop that starts ([`open_known`]) sends `KK` message 1 (`e, es, ss`).
//! 2. The laptop that answers ([`accept_known`]) sends `KK` message 2 (`e, ee, se`). Only a
//!    laptop holding the private half of the key this side remembered can make it.
//! 3. The starting laptop sends a sealed confirmation ([`KnownOpening::finish`]). `KK` message 1
//!    can be replayed by anyone who saw it, so the answering laptop treats nothing as
//!    authenticated until this confirmation proves the other side read message 2 live
//!    ([`KnownAccepting::confirm`]).
//!
//! Every message carries the protocol version and its own message kind, is size-limited, and the
//! caller's context (what both sides agreed this session is for) is bound in as the prologue,
//! with both roles. The X25519 keys come from `curve25519-dalek` (RFC 7748) and the system's
//! random source; nothing here is homemade cryptography.

use std::time::{Duration, Instant};

use curve25519_dalek::montgomery::MontgomeryPoint;
use snow::{Builder, HandshakeState};
use zeroize::Zeroizing;

use crate::{
    Deadline, MAX_MESSAGE_LEN, PairingError, Transport, finish_handshake, frame, parse_frame,
};

const KNOWN_PARAMS: &str = "Noise_KK_25519_ChaChaPoly_BLAKE2s";
const KNOWN_PROLOGUE_LABEL: &[u8] = b"pctwin/v1/known-prologue";
const KNOWN_ROLES: &[u8] = b"pctwin/v1/known roles: the laptop that connects starts";
const KNOWN_READY_LABEL: &[u8] = b"pctwin/v1/known-ready";
const KIND_KNOWN_1: u8 = 12;
const KIND_KNOWN_2: u8 = 13;
const KIND_KNOWN_3: u8 = 14;
/// Longest context a caller may bind in.
pub const MAX_CONTEXT_LEN: usize = 64;
/// How long each side waits for the next message of a known-keys handshake.
pub const KNOWN_TIMEOUT: Duration = Duration::from_secs(10);

/// One laptop's long-term key pair for one remembered connection. The private half is wiped
/// from memory when dropped, and is kept only in the system's key store.
pub struct StaticKeys {
    private: Zeroizing<[u8; 32]>,
    public: [u8; 32],
}

impl std::fmt::Debug for StaticKeys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("StaticKeys(..)")
    }
}

impl StaticKeys {
    /// A new key pair from the system's random source.
    pub fn generate() -> Result<Self, PairingError> {
        let mut private = Zeroizing::new([0u8; 32]);
        getrandom::fill(private.as_mut()).map_err(|_| PairingError::Random)?;
        Ok(Self::from_private(private))
    }

    /// The key pair whose private half is `private` (as kept in the system's key store); the
    /// public half is worked out from it, so the two can never disagree.
    pub fn from_private(private: Zeroizing<[u8; 32]>) -> Self {
        let public = MontgomeryPoint::mul_base_clamped(*private).to_bytes();
        Self { private, public }
    }

    /// The public half: the only half ever sent to the other laptop.
    pub fn public(&self) -> [u8; 32] {
        self.public
    }

    /// The private half, only to keep it in the system's key store.
    pub fn private(&self) -> &Zeroizing<[u8; 32]> {
        &self.private
    }
}

/// A public key the other laptop sent: refused when it is one that makes every exchange with it
/// predictable (an all-zero key, or another of the few low-order points).
pub fn check_public(public: &[u8; 32]) -> Result<(), PairingError> {
    // Multiplying by the cofactor (8) sends every low-order point, and only those, to zero.
    let point = MontgomeryPoint(*public);
    let eight = MontgomeryPoint::mul_clamped(point, {
        let mut s = [0u8; 32];
        s[0] = 8;
        s
    });
    // `mul_clamped` sets bit 254 and clears the low bits; clamping 8 gives 2^254, a multiple of
    // the cofactor, so a low-order point still lands on zero and every other point does not.
    if eight.to_bytes() == [0u8; 32] {
        return Err(PairingError::InvalidKey);
    }
    Ok(())
}

fn prologue(context: &[u8]) -> Result<Vec<u8>, PairingError> {
    if context.len() > MAX_CONTEXT_LEN {
        return Err(PairingError::TooLarge);
    }
    let mut p =
        Vec::with_capacity(KNOWN_PROLOGUE_LABEL.len() + KNOWN_ROLES.len() + 2 + context.len());
    p.extend_from_slice(KNOWN_PROLOGUE_LABEL);
    p.extend_from_slice(KNOWN_ROLES);
    p.push(crate::PROTOCOL_VERSION);
    p.push(u8::try_from(context.len()).map_err(|_| PairingError::TooLarge)?);
    p.extend_from_slice(context);
    Ok(p)
}

fn builder<'a>(
    local: &'a StaticKeys,
    remote: &'a [u8; 32],
    prologue: &'a [u8],
) -> Result<Builder<'a>, PairingError> {
    let params = KNOWN_PARAMS
        .parse()
        .map_err(|_| PairingError::HandshakeFailed)?;
    Builder::new(params)
        .local_private_key(local.private.as_ref())
        .and_then(|b| b.remote_public_key(remote))
        .and_then(|b| b.prologue(prologue))
        .map_err(|_| PairingError::HandshakeFailed)
}

/// The laptop that connects, after sending message 1, waiting for message 2.
pub struct KnownOpening {
    hs: HandshakeState,
    deadline: Deadline,
}

/// The laptop that connects: starts a known-keys session with the laptop whose remembered
/// public key is `remote`, using this connection's own `local` keys, for `context`. Returns
/// message 1.
pub fn open_known(
    local: &StaticKeys,
    remote: &[u8; 32],
    context: &[u8],
    now: Instant,
) -> Result<(KnownOpening, Vec<u8>), PairingError> {
    check_public(remote)?;
    let prologue = prologue(context)?;
    let mut hs = builder(local, remote, &prologue)?
        .build_initiator()
        .map_err(|_| PairingError::HandshakeFailed)?;
    let mut buf = vec![0u8; MAX_MESSAGE_LEN];
    let n = hs
        .write_message(&[], &mut buf)
        .map_err(|_| PairingError::HandshakeFailed)?;
    buf.truncate(n);
    Ok((
        KnownOpening {
            hs,
            deadline: Deadline::after(now, KNOWN_TIMEOUT),
        },
        frame(KIND_KNOWN_1, &buf),
    ))
}

impl KnownOpening {
    /// Handles message 2 at time `now`: the session's link, and message 3 (the sealed
    /// confirmation) to send at once.
    pub fn finish(
        mut self,
        msg2: &[u8],
        now: Instant,
    ) -> Result<(Transport, Vec<u8>), PairingError> {
        if self.deadline.passed(now) {
            return Err(PairingError::Expired);
        }
        let body = parse_frame(msg2, KIND_KNOWN_2)?;
        let mut out = vec![0u8; MAX_MESSAGE_LEN];
        let n = self
            .hs
            .read_message(body, &mut out)
            .map_err(|_| PairingError::HandshakeFailed)?;
        if n != 0 {
            return Err(PairingError::Malformed);
        }
        let (transport, _hash) = finish_handshake(self.hs)?;
        let mut transport = Transport(transport);
        let ready = transport.seal(KNOWN_READY_LABEL)?;
        Ok((transport, frame(KIND_KNOWN_3, &ready)))
    }
}

/// The answering laptop, after sending message 2, waiting for the confirmation.
pub struct KnownAccepting {
    transport: Transport,
    deadline: Deadline,
}

/// The answering laptop: handles message 1 from the laptop whose remembered public key is
/// `remote`, with this connection's own `local` keys, for `context`. Returns message 2. Nothing
/// is authenticated until [`KnownAccepting::confirm`].
pub fn accept_known(
    local: &StaticKeys,
    remote: &[u8; 32],
    context: &[u8],
    msg1: &[u8],
    now: Instant,
) -> Result<(KnownAccepting, Vec<u8>), PairingError> {
    check_public(remote)?;
    let body = parse_frame(msg1, KIND_KNOWN_1)?;
    let prologue = prologue(context)?;
    let mut hs = builder(local, remote, &prologue)?
        .build_responder()
        .map_err(|_| PairingError::HandshakeFailed)?;
    let mut out = vec![0u8; MAX_MESSAGE_LEN];
    let n = hs
        .read_message(body, &mut out)
        .map_err(|_| PairingError::HandshakeFailed)?;
    if n != 0 {
        return Err(PairingError::Malformed);
    }
    let mut buf = vec![0u8; MAX_MESSAGE_LEN];
    let n = hs
        .write_message(&[], &mut buf)
        .map_err(|_| PairingError::HandshakeFailed)?;
    buf.truncate(n);
    let (transport, _hash) = finish_handshake(hs)?;
    Ok((
        KnownAccepting {
            transport: Transport(transport),
            deadline: Deadline::after(now, KNOWN_TIMEOUT),
        },
        frame(KIND_KNOWN_2, &buf),
    ))
}

impl KnownAccepting {
    /// Handles message 3 at time `now`: the link, only once the other side proved it is live.
    pub fn confirm(mut self, msg3: &[u8], now: Instant) -> Result<Transport, PairingError> {
        if self.deadline.passed(now) {
            return Err(PairingError::Expired);
        }
        let body = parse_frame(msg3, KIND_KNOWN_3)?;
        let ready = self
            .transport
            .open(body)
            .map_err(|_| PairingError::HandshakeFailed)?;
        if ready.as_slice() != KNOWN_READY_LABEL {
            return Err(PairingError::HandshakeFailed);
        }
        Ok(self.transport)
    }
}
