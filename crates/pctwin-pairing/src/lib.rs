//! PCTwin pairing: turns a short one-time code into an authenticated, encrypted link
//! between two laptops (Security Design, Part A).
//!
//! # How it works
//!
//! 1. The sender (old laptop) shows a six-digit [`PairingCode`]. [`RotatingSender`] replaces it
//!    every minute; the previous code keeps a short grace period. Codes alternate between an even
//!    and an odd last digit, so the receiver can name which live code it holds
//!    ([`PairingCode::parity`], [`RotatingSender::message_1_for`]) at the cost of one bit. Each code carries a random
//!    session tag in the messages, so a reply is matched to the code it was made from and a stray
//!    reply never burns the current code.
//! 2. Each code gets exactly one attempt. Failed attempts are budgeted: after each failure the
//!    sender pauses before showing a new code (1, 2, 4, 8 seconds), and after
//!    [`MAX_FAILED_ATTEMPTS`] failures in a row pairing locks until the person on the sender
//!    chooses to start again ([`RotatingSender::unlock`]).
//! 3. SPAKE2 (Ed25519 group, asymmetric roles) turns the code into a shared secret that an
//!    eavesdropper cannot use to test guesses offline. Degenerate (small-order or mixed-torsion)
//!    SPAKE2 points are rejected before use.
//! 4. A Noise `NNpsk0` handshake builds the encrypted link with fresh ephemeral keys
//!    (forward secrecy). The SPAKE2 secret is the pre-shared key; the protocol version, the session
//!    tag and both SPAKE2 messages are bound in as the Noise prologue.
//! 5. Both sides derive a two-digit match number from the final handshake hash and a random
//!    value the receiver locked in with a hash commitment inside message 2 and reveals only after
//!    the sender's last handshake message (message 4). Because neither side can see the other's
//!    final input before fixing its own, nobody in the middle can try keys until the two laptops
//!    happen to show the same number; each attempt is a blind 1-in-90 guess. The receiver
//!    (new laptop) shows the number; the sender (old laptop) offers three numbers (the right one and two
//!    random decoys) and the person picks, on the old laptop, the one shown on the new laptop.
//! 6. Only that pick on the old laptop unlocks it: nothing the other side sends can, so someone
//!    who knows the code still cannot reach the old laptop's files without the person at the old
//!    laptop choosing the right number. The sender then seals an approval (message 5) to the
//!    receiver, which unlocks the receiver. Only then does either side get a usable [`Paired`] link.
//! 7. Extra lanes need no new code: the new laptop opens one with [`Paired::open_lane`] on a new
//!    connection, and the old laptop accepts it with [`Paired::accept_lane`]. Each lane runs its
//!    own Noise `NNpsk0` handshake with fresh ephemeral keys, so every lane has its own keys and
//!    counters; its pre-shared key is derived (HKDF) from the pairing's key, the final handshake
//!    hash and the lane number. Numbers are used once, and the old laptop uses a lane only after
//!    a sealed confirmation proves the opener is live.
//!
//! Every message carries the protocol version and a message kind, and is size-limited. Malformed
//! input returns an error instead of panicking. Messages that fail before authentication hand the
//! waiting state back ([`Rejected`]), so junk cannot cancel a pairing in progress.
//!
//! # Known limits
//!
//! - The human decision protects the sender, which holds the files. Someone who knows the code can
//!   pose as a sender and be approved by a receiver; the receiver's safety gate and the person's
//!   review of the plan then guard what that fake sender offers.
//! - Someone who knows the code and sits between both laptops wins only if two independent
//!   numbers collide (1 in 90 per attempt), and each attempt needs the person to type a code again.
//! - `spake2` states it has had no independent audit and is probably not constant-time; `snow`
//!   has had no formal audit. Neither zeroizes its internal secrets on drop.
//! - Session tags are public, so anyone who sees message 1 can spend that code's single attempt
//!   with a junk reply (and repeated junk replies trigger the lockout). Binding attempts to one
//!   network connection belongs to the transport layer.
//! - Time is supplied by the caller. `Instant` does not count suspend on every platform, so the
//!   app must rotate the code when the laptop wakes from sleep.
//! - Starting a session panics if the operating system's random source fails, as SPAKE2 does.

use std::collections::BTreeSet;
use std::fmt;
use std::ops::RangeInclusive;
use std::time::{Duration, Instant};

use curve25519_dalek::edwards::CompressedEdwardsY;
use hkdf::Hkdf;
use sha2::{Digest, Sha256};
use snow::{Builder, HandshakeState, TransportState};
use spake2::{Ed25519Group, Identity, Password, Spake2};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

/// Version of the pairing protocol carried in every message.
pub const PROTOCOL_VERSION: u8 = 1;
/// How long a code is shown before it is replaced.
pub const CODE_LIFETIME: Duration = Duration::from_secs(60);
/// How long a replaced code still works, for someone who was typing it when it changed.
pub const ROTATION_GRACE: Duration = Duration::from_secs(15);
/// How long either side waits, after the handshake, for the person's pick and the approval.
pub const CONFIRMATION_TIMEOUT: Duration = Duration::from_secs(180);
/// How long the receiver waits for message 3 after sending message 2.
pub const REPLY_TIMEOUT: Duration = Duration::from_secs(30);
/// Failed attempts in a row before pairing locks until the person on the sender starts again.
pub const MAX_FAILED_ATTEMPTS: u32 = 5;
/// Largest pairing message accepted, in bytes.
pub const MAX_MESSAGE_LEN: usize = 512;
/// Range of the match number shown during confirmation.
pub const MATCH_NUMBER_RANGE: RangeInclusive<u8> = 10..=99;
/// Most extra lanes one paired session may open (numbers 1 to this, each used once).
pub const MAX_LANE_OPENINGS: u32 = 256;

const NOISE_PARAMS: &str = "Noise_NNpsk0_25519_ChaChaPoly_BLAKE2s";
const SPAKE2_MSG_LEN: usize = 33;
const SESSION_ID_LEN: usize = 8;
const ID_SENDER: &[u8] = b"pctwin/v1/sender";
const ID_RECEIVER: &[u8] = b"pctwin/v1/receiver";
const PROLOGUE_LABEL: &[u8] = b"pctwin/v1/pairing-prologue";
const PSK_SALT: &[u8] = b"pctwin/v1/pairing-psk";
const MATCH_SALT: &[u8] = b"pctwin/v1/match-number";
const APPROVE_LABEL: &[u8] = b"pctwin/v1/approved";
const COMMIT_LABEL: &[u8] = b"pctwin/v1/number-commitment";
const LANE_SECRET_SALT: &[u8] = b"pctwin/v1/lane-secret";
const LANE_PSK_SALT: &[u8] = b"pctwin/v1/lane-psk";
const LANE_PROLOGUE_LABEL: &[u8] = b"pctwin/v1/lane-prologue";
const LANE_READY_LABEL: &[u8] = b"pctwin/v1/lane-ready";
const LANE_NUMBER_LEN: usize = 4;
const NONCE_LEN: usize = 32;
const COMMIT_LEN: usize = 32;
const MAX_CHOICE_DRAWS: u32 = 64;
const AEAD_TAG_LEN: usize = 16;
const NOISE_MAX: usize = 65_535;

const KIND_SPAKE_A: u8 = 1;
const KIND_SPAKE_B_NOISE_1: u8 = 2;
const KIND_NOISE_2: u8 = 3;
const KIND_REVEAL: u8 = 4;
const KIND_APPROVE: u8 = 5;
const KIND_LANE_1: u8 = 6;
const KIND_LANE_2: u8 = 7;
const KIND_LANE_3: u8 = 8;

/// Why pairing or the encrypted link failed. Messages say what to do, not which byte was wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PairingError {
    #[error("the code must be six digits")]
    InvalidCode,
    #[error("the code has expired; show a new code")]
    Expired,
    #[error("that code is no longer active; type the code shown now")]
    UnknownSession,
    #[error("too many tries; wait a moment for a new code")]
    CoolingDown,
    #[error("too many tries; start again on the old laptop")]
    Locked,
    #[error("the other laptop uses a different PCTwin version ({0})")]
    UnsupportedVersion(u8),
    #[error("a pairing message was not in the expected form")]
    Malformed,
    #[error("the other laptop sent an invalid pairing key")]
    InvalidKey,
    #[error("the laptops could not pair; check the code and try a new one")]
    HandshakeFailed,
    #[error("that number does not match the one on the other laptop")]
    WrongNumber,
    #[error("the system's secure random source failed")]
    Random,
    #[error("the data is too large to send in one piece")]
    TooLarge,
    #[error("received data failed its integrity check")]
    Transport,
    #[error("an extra connection was refused; the move carries on over the others")]
    LaneRefused,
}

/// A failed step. When the failure happened before anything was authenticated (for example a
/// junk or truncated message), the waiting state is handed back so pairing can continue.
pub struct Rejected<S> {
    error: PairingError,
    retry: Option<Box<S>>,
}

impl<S> Rejected<S> {
    fn keep(error: PairingError, state: S) -> Self {
        Self {
            error,
            retry: Some(Box::new(state)),
        }
    }

    fn end(error: PairingError) -> Self {
        Self { error, retry: None }
    }

    /// Why the step failed.
    pub fn error(&self) -> PairingError {
        self.error
    }

    /// The waiting state, if the failure did not end the pairing.
    pub fn into_retry(self) -> Option<S> {
        self.retry.map(|b| *b)
    }
}

impl<S> From<Rejected<S>> for PairingError {
    fn from(r: Rejected<S>) -> Self {
        r.error
    }
}

impl<S> fmt::Debug for Rejected<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Rejected")
            .field("error", &self.error)
            .field("can_retry", &self.retry.is_some())
            .finish()
    }
}

/// A six-digit, one-time pairing code. Its digits are wiped from memory when dropped.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct PairingCode {
    digits: [u8; 6],
}

impl PairingCode {
    /// Generates a uniformly random code from the operating system's secure random source.
    pub fn generate() -> Result<Self, PairingError> {
        // Rejection sampling: 4_294_000_000 is the largest multiple of 1_000_000 below 2^32.
        Self::generate_from(4_294_000_000, |n| n % 1_000_000)
    }

    /// Generates a random code whose last digit is even (`parity` 0) or odd (`parity` 1).
    fn generate_with_parity(parity: u8) -> Result<Self, PairingError> {
        let parity = u32::from(parity & 1);
        // Rejection sampling: 4_294_500_000 is the largest multiple of 500_000 below 2^32.
        Self::generate_from(4_294_500_000, |n| (n % 500_000) * 2 + parity)
    }

    fn generate_from(limit: u32, to_value: impl Fn(u32) -> u32) -> Result<Self, PairingError> {
        loop {
            let mut bytes = [0u8; 4];
            getrandom::fill(&mut bytes).map_err(|_| PairingError::Random)?;
            let n = u32::from_le_bytes(bytes);
            bytes.zeroize();
            if n < limit {
                let mut value = to_value(n);
                let mut digits = [b'0'; 6];
                for slot in digits.iter_mut().rev() {
                    // `value % 10` is always 0..=9, so the cast cannot truncate.
                    *slot = b'0' + (value % 10) as u8;
                    value /= 10;
                }
                return Ok(Self { digits });
            }
        }
    }

    /// Reads a code typed by a person. ASCII spaces are ignored; anything else must be six ASCII digits.
    pub fn parse(typed: &str) -> Result<Self, PairingError> {
        let mut digits = [0u8; 6];
        let mut count = 0usize;
        for c in typed.chars() {
            if c == ' ' {
                continue;
            }
            if !c.is_ascii_digit() || count == 6 {
                digits.zeroize();
                return Err(PairingError::InvalidCode);
            }
            // `c` is an ASCII digit, so it fits in one byte.
            digits[count] = c as u8;
            count += 1;
        }
        if count != 6 {
            digits.zeroize();
            return Err(PairingError::InvalidCode);
        }
        Ok(Self { digits })
    }

    /// The six digits, for display on the sender.
    pub fn digits(&self) -> String {
        self.digits.iter().map(|&d| d as char).collect()
    }

    /// 0 if the last digit is even, 1 if odd. Not secret: the receiver sends it so the sender
    /// can pick the matching live code.
    pub fn parity(&self) -> u8 {
        (self.digits[5] - b'0') & 1
    }

    fn password(&self) -> Password {
        Password::new(self.digits)
    }
}

impl fmt::Debug for PairingCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PairingCode(******)")
    }
}

/// Sender side, after showing the code and sending message 1.
pub struct SenderSession {
    id: [u8; SESSION_ID_LEN],
    spake: Spake2<Ed25519Group>,
    spake_a: Vec<u8>,
    deadline: Instant,
}

impl SenderSession {
    /// Starts pairing with the given code at time `now`. Returns message 1 to send.
    /// The code is accepted until `now + CODE_LIFETIME + ROTATION_GRACE`.
    ///
    /// # Panics
    ///
    /// Panics if the operating system's secure random source fails, as SPAKE2 itself does.
    pub fn start(code: &PairingCode, now: Instant) -> (Self, Vec<u8>) {
        let mut id = [0u8; SESSION_ID_LEN];
        #[allow(clippy::expect_used)]
        getrandom::fill(&mut id).expect("operating system random source failed");
        let (spake, spake_a) = Spake2::<Ed25519Group>::start_a(
            &code.password(),
            &Identity::new(ID_SENDER),
            &Identity::new(ID_RECEIVER),
        );
        let mut payload = id.to_vec();
        payload.extend_from_slice(&spake_a);
        let msg1 = frame(KIND_SPAKE_A, &payload);
        (
            Self {
                id,
                spake,
                spake_a,
                deadline: now + CODE_LIFETIME + ROTATION_GRACE,
            },
            msg1,
        )
    }

    /// Handles message 2 at time `now`. Consumes the session: a code gets exactly one attempt.
    /// Returns the state waiting for the receiver's reveal, and message 3 to send.
    pub fn receive(
        self,
        msg2: &[u8],
        now: Instant,
    ) -> Result<(SenderAwaitingReveal, Vec<u8>), PairingError> {
        if now > self.deadline {
            return Err(PairingError::Expired);
        }
        let payload = parse_frame(msg2, KIND_SPAKE_B_NOISE_1)?;
        if payload.len() <= SESSION_ID_LEN + SPAKE2_MSG_LEN {
            return Err(PairingError::Malformed);
        }
        let (id, rest) = payload.split_at(SESSION_ID_LEN);
        if id != self.id {
            return Err(PairingError::UnknownSession);
        }
        let (spake_b, noise_1) = rest.split_at(SPAKE2_MSG_LEN);
        check_spake_point(spake_b)?;
        let key = Zeroizing::new(
            self.spake
                .finish(spake_b)
                .map_err(|_| PairingError::HandshakeFailed)?,
        );
        let psk = derive_psk(&key)?;
        let prologue = prologue(&self.id, &self.spake_a, spake_b);
        let mut hs = noise_builder(&psk, &prologue)?
            .build_responder()
            .map_err(|_| PairingError::HandshakeFailed)?;

        let mut buf = [0u8; 128];
        let read = hs
            .read_message(noise_1, &mut buf)
            .map_err(|_| PairingError::HandshakeFailed)?;
        let commitment: [u8; COMMIT_LEN] = buf[..read]
            .try_into()
            .map_err(|_| PairingError::Malformed)?;
        let written = hs
            .write_message(&[], &mut buf)
            .map_err(|_| PairingError::HandshakeFailed)?;
        let msg3 = frame(KIND_NOISE_2, &buf[..written]);

        let (transport, handshake_hash) = finish_handshake(hs)?;
        let lane_secret = derive_lane_secret(&psk, &handshake_hash)?;
        Ok((
            SenderAwaitingReveal {
                transport,
                lane_secret,
                handshake_hash,
                commitment,
                deadline: now + CONFIRMATION_TIMEOUT,
            },
            msg3,
        ))
    }
}

/// Sender side, after message 3, waiting for the receiver to reveal the value it committed to.
pub struct SenderAwaitingReveal {
    transport: TransportState,
    lane_secret: Zeroizing<[u8; 32]>,
    handshake_hash: Vec<u8>,
    commitment: [u8; COMMIT_LEN],
    deadline: Instant,
}

impl SenderAwaitingReveal {
    /// When the sender stops waiting for the reveal.
    pub fn deadline(&self) -> Instant {
        self.deadline
    }

    /// Handles message 4 at time `now`. Returns the three numbers to offer the person. Junk,
    /// truncated or forged messages hand the waiting state back; a reveal that does not match the
    /// receiver's commitment, or a missed deadline, ends the pairing.
    pub fn receive_reveal(
        mut self,
        msg4: &[u8],
        now: Instant,
    ) -> Result<SenderChoosing, Rejected<Self>> {
        if now > self.deadline {
            return Err(Rejected::end(PairingError::Expired));
        }
        let payload = match parse_frame(msg4, KIND_REVEAL) {
            Ok(p) => p,
            Err(e) => return Err(Rejected::keep(e, self)),
        };
        let mut buf = Zeroizing::new(vec![0u8; payload.len()]);
        // A message that fails authentication does not advance the transport's nonce.
        let read = match self.transport.read_message(payload, &mut buf) {
            Ok(n) => n,
            Err(_) => return Err(Rejected::keep(PairingError::HandshakeFailed, self)),
        };
        let nonce = &buf[..read];
        // The commitment covers the exact bytes, so a reveal of any other length fails here too.
        if commit_to(nonce) != self.commitment {
            return Err(Rejected::end(PairingError::HandshakeFailed));
        }
        let correct = match_number(&self.handshake_hash, nonce).map_err(Rejected::end)?;
        let choices = offer_choices(correct).map_err(Rejected::end)?;
        Ok(SenderChoosing {
            transport: self.transport,
            lane_secret: self.lane_secret,
            correct,
            choices,
            deadline: self.deadline,
        })
    }
}

/// Sender side, offering three numbers. The person picks the one shown on the receiver.
/// This pick, made on the sender, is the only thing that unlocks the sender.
pub struct SenderChoosing {
    transport: TransportState,
    lane_secret: Zeroizing<[u8; 32]>,
    correct: u8,
    choices: [u8; 3],
    deadline: Instant,
}

impl SenderChoosing {
    /// The three numbers to show on the sender, in display order. Exactly one matches the receiver.
    pub fn choices(&self) -> [u8; 3] {
        self.choices
    }

    /// When the sender stops waiting for the person's pick.
    pub fn deadline(&self) -> Instant {
        self.deadline
    }

    /// Records the person's pick at time `now`. The right number unlocks the sender and returns
    /// message 5, the sealed approval for the receiver. Any other number, or a pick after the
    /// deadline, cancels pairing. To cancel without picking (for example when none of the three
    /// numbers matches the receiver), drop this value.
    pub fn choose(mut self, picked: u8, now: Instant) -> Result<(Paired, Vec<u8>), PairingError> {
        if now > self.deadline {
            return Err(PairingError::Expired);
        }
        if picked != self.correct {
            return Err(PairingError::WrongNumber);
        }
        let mut plain = APPROVE_LABEL.to_vec();
        plain.push(picked);
        let mut buf = [0u8; 64];
        let written = self
            .transport
            .write_message(&plain, &mut buf)
            .map_err(|_| PairingError::HandshakeFailed)?;
        let msg5 = frame(KIND_APPROVE, &buf[..written]);
        Ok((
            Paired::new(Transport(self.transport), self.lane_secret, false),
            msg5,
        ))
    }
}

/// Receiver side, after the person typed the code and message 2 was sent.
pub struct ReceiverSession {
    hs: HandshakeState,
    psk: Zeroizing<[u8; 32]>,
    nonce: Zeroizing<[u8; NONCE_LEN]>,
    deadline: Instant,
}

impl ReceiverSession {
    /// Handles message 1 using the typed code at time `now`. Returns the session and message 2
    /// to send. Message 2 carries a commitment to a fresh random value, revealed in message 4.
    pub fn respond(
        code: &PairingCode,
        msg1: &[u8],
        now: Instant,
    ) -> Result<(Self, Vec<u8>), PairingError> {
        let payload = parse_frame(msg1, KIND_SPAKE_A)?;
        if payload.len() != SESSION_ID_LEN + SPAKE2_MSG_LEN {
            return Err(PairingError::Malformed);
        }
        let (id, spake_a) = payload.split_at(SESSION_ID_LEN);
        check_spake_point(spake_a)?;
        let (spake, spake_b) = Spake2::<Ed25519Group>::start_b(
            &code.password(),
            &Identity::new(ID_SENDER),
            &Identity::new(ID_RECEIVER),
        );
        let key = Zeroizing::new(
            spake
                .finish(spake_a)
                .map_err(|_| PairingError::HandshakeFailed)?,
        );
        let psk = derive_psk(&key)?;
        let prologue = prologue(id, spake_a, &spake_b);
        let mut hs = noise_builder(&psk, &prologue)?
            .build_initiator()
            .map_err(|_| PairingError::HandshakeFailed)?;

        let mut nonce = Zeroizing::new([0u8; NONCE_LEN]);
        getrandom::fill(nonce.as_mut()).map_err(|_| PairingError::Random)?;
        let mut buf = [0u8; 128];
        let written = hs
            .write_message(&commit_to(nonce.as_ref()), &mut buf)
            .map_err(|_| PairingError::HandshakeFailed)?;
        let mut payload = id.to_vec();
        payload.extend_from_slice(&spake_b);
        payload.extend_from_slice(&buf[..written]);
        Ok((
            Self {
                hs,
                psk,
                nonce,
                deadline: now + REPLY_TIMEOUT,
            },
            frame(KIND_SPAKE_B_NOISE_1, &payload),
        ))
    }

    /// Handles message 3 at time `now`. Returns the number to show on the receiver and message 4
    /// (the reveal) to send. Message 3 after [`REPLY_TIMEOUT`] ends the pairing. Any message that fails
    /// before the handshake completes (junk, truncated, forged) hands the session back: `snow`
    /// restores its handshake state on a failed read, so the genuine message 3 still works.
    pub fn receive(
        mut self,
        msg3: &[u8],
        now: Instant,
    ) -> Result<(ReceiverAwaitingApproval, Vec<u8>), Rejected<Self>> {
        if now > self.deadline {
            return Err(Rejected::end(PairingError::Expired));
        }
        let payload = match parse_frame(msg3, KIND_NOISE_2) {
            Ok(p) => p,
            Err(e) => return Err(Rejected::keep(e, self)),
        };
        let mut buf = [0u8; 128];
        let read = match self.hs.read_message(payload, &mut buf) {
            Ok(n) => n,
            Err(_) => return Err(Rejected::keep(PairingError::HandshakeFailed, self)),
        };
        if read != 0 {
            return Err(Rejected::end(PairingError::Malformed));
        }
        let (mut transport, handshake_hash) = finish_handshake(self.hs).map_err(Rejected::end)?;
        let lane_secret = derive_lane_secret(&self.psk, &handshake_hash).map_err(Rejected::end)?;
        let match_number =
            match_number(&handshake_hash, self.nonce.as_ref()).map_err(Rejected::end)?;
        let mut sealed = [0u8; NONCE_LEN + AEAD_TAG_LEN];
        let written = transport
            .write_message(self.nonce.as_ref(), &mut sealed)
            .map_err(|_| Rejected::end(PairingError::HandshakeFailed))?;
        Ok((
            ReceiverAwaitingApproval {
                transport,
                lane_secret,
                match_number,
                deadline: now + CONFIRMATION_TIMEOUT,
            },
            frame(KIND_REVEAL, &sealed[..written]),
        ))
    }
}

/// Receiver side, showing its number and waiting for the sender's approval.
pub struct ReceiverAwaitingApproval {
    transport: TransportState,
    lane_secret: Zeroizing<[u8; 32]>,
    match_number: u8,
    deadline: Instant,
}

impl ReceiverAwaitingApproval {
    /// The number to show on the receiver's screen.
    pub fn match_number(&self) -> u8 {
        self.match_number
    }

    /// When the receiver stops waiting for the approval.
    pub fn deadline(&self) -> Instant {
        self.deadline
    }

    /// Handles message 5 at time `now`. Succeeds only for a genuine approval from this session
    /// carrying the receiver's own number. Junk, truncated or forged messages hand the waiting
    /// state back; a wrong number, a malformed approval or a missed deadline ends the pairing.
    pub fn receive_approval(mut self, msg5: &[u8], now: Instant) -> Result<Paired, Rejected<Self>> {
        if now > self.deadline {
            return Err(Rejected::end(PairingError::Expired));
        }
        let payload = match parse_frame(msg5, KIND_APPROVE) {
            Ok(p) => p,
            Err(e) => return Err(Rejected::keep(e, self)),
        };
        let mut buf = Zeroizing::new(vec![0u8; payload.len()]);
        // A message that fails authentication does not advance the transport's nonce, so the
        // waiting state stays usable for the genuine approval.
        let read = match self.transport.read_message(payload, &mut buf) {
            Ok(n) => n,
            Err(_) => return Err(Rejected::keep(PairingError::HandshakeFailed, self)),
        };
        let plain = &buf[..read];
        match plain.split_last() {
            Some((&picked, label)) if label == APPROVE_LABEL => {
                if picked == self.match_number {
                    Ok(Paired::new(
                        Transport(self.transport),
                        self.lane_secret,
                        true,
                    ))
                } else {
                    Err(Rejected::end(PairingError::WrongNumber))
                }
            }
            _ => Err(Rejected::end(PairingError::HandshakeFailed)),
        }
    }
}

/// A confirmed pairing with its encrypted link.
pub struct Paired {
    transport: Transport,
    lanes: LaneKeys,
}

impl Paired {
    fn new(transport: Transport, lane_secret: Zeroizing<[u8; 32]>, opens_lanes: bool) -> Self {
        Self {
            transport,
            lanes: LaneKeys {
                lane_secret,
                opens_lanes,
                next_lane: 1,
                used_lanes: BTreeSet::new(),
            },
        }
    }

    /// The encrypted link to the other laptop.
    pub fn transport_mut(&mut self) -> &mut Transport {
        &mut self.transport
    }

    /// New laptop: see [`LaneKeys::open_lane`].
    pub fn open_lane(&mut self, now: Instant) -> Result<(LaneOpening, Vec<u8>), PairingError> {
        self.lanes.open_lane(now)
    }

    /// Old laptop: see [`LaneKeys::accept_lane`].
    pub fn accept_lane(
        &mut self,
        msg1: &[u8],
        now: Instant,
    ) -> Result<(LaneAccepting, Vec<u8>), PairingError> {
        self.lanes.accept_lane(msg1, now)
    }

    /// The encrypted link and the keys for extra lanes, held apart: the link stays busy carrying
    /// the move while lanes are opened (new laptop) or accepted (old laptop) beside it.
    pub fn into_parts(self) -> (Transport, LaneKeys) {
        (self.transport, self.lanes)
    }
}

/// What a paired session needs to open (new laptop) or accept (old laptop) extra lanes: a secret
/// bound to exactly this pairing, and which lane numbers are used. Wiped from memory when dropped.
pub struct LaneKeys {
    lane_secret: Zeroizing<[u8; 32]>,
    opens_lanes: bool,
    next_lane: u32,
    used_lanes: BTreeSet<u32>,
}

impl LaneKeys {
    /// New laptop: starts an extra lane at time `now`, returning lane message 1 to send on a new
    /// connection to the old laptop. Each call uses the next lane number, never one used before,
    /// even if that lane failed. Only the new laptop opens lanes.
    pub fn open_lane(&mut self, now: Instant) -> Result<(LaneOpening, Vec<u8>), PairingError> {
        if !self.opens_lanes || self.next_lane > MAX_LANE_OPENINGS {
            return Err(PairingError::LaneRefused);
        }
        let lane = self.next_lane;
        self.next_lane += 1;
        let psk = derive_lane_psk(&self.lane_secret, lane)?;
        let prologue = lane_prologue(lane);
        let mut hs = noise_builder(&psk, &prologue)?
            .build_initiator()
            .map_err(|_| PairingError::HandshakeFailed)?;
        let mut buf = [0u8; 128];
        let written = hs
            .write_message(&[], &mut buf)
            .map_err(|_| PairingError::HandshakeFailed)?;
        let mut payload = lane.to_be_bytes().to_vec();
        payload.extend_from_slice(&buf[..written]);
        Ok((
            LaneOpening {
                hs,
                lane,
                deadline: now + REPLY_TIMEOUT,
            },
            frame(KIND_LANE_1, &payload),
        ))
    }

    /// Old laptop: handles lane message 1 from a new connection at time `now`, returning lane
    /// message 2. Refuses lane numbers outside 1 to [`MAX_LANE_OPENINGS`] and any number already
    /// used. A number is spent only once its message proves the session's keys, so junk from a
    /// stranger never uses one up. The lane is usable only after [`LaneAccepting::confirm`].
    pub fn accept_lane(
        &mut self,
        msg1: &[u8],
        now: Instant,
    ) -> Result<(LaneAccepting, Vec<u8>), PairingError> {
        if self.opens_lanes {
            return Err(PairingError::LaneRefused);
        }
        let payload = parse_frame(msg1, KIND_LANE_1)?;
        let (number, noise_1) = payload
            .split_at_checked(LANE_NUMBER_LEN)
            .ok_or(PairingError::Malformed)?;
        let lane = u32::from_be_bytes(number.try_into().map_err(|_| PairingError::Malformed)?);
        if !(1..=MAX_LANE_OPENINGS).contains(&lane) || self.used_lanes.contains(&lane) {
            return Err(PairingError::LaneRefused);
        }
        let psk = derive_lane_psk(&self.lane_secret, lane)?;
        let prologue = lane_prologue(lane);
        let mut hs = noise_builder(&psk, &prologue)?
            .build_responder()
            .map_err(|_| PairingError::HandshakeFailed)?;
        let mut buf = [0u8; 128];
        let read = hs
            .read_message(noise_1, &mut buf)
            .map_err(|_| PairingError::HandshakeFailed)?;
        if read != 0 {
            return Err(PairingError::Malformed);
        }
        // The message proved the session's keys: this number is spent, even if the lane never
        // completes, so a copy of the message cannot open a second lane.
        self.used_lanes.insert(lane);
        let written = hs
            .write_message(&[], &mut buf)
            .map_err(|_| PairingError::HandshakeFailed)?;
        let (transport, _) = finish_handshake(hs)?;
        Ok((
            LaneAccepting {
                transport,
                lane,
                deadline: now + REPLY_TIMEOUT,
            },
            frame(KIND_LANE_2, &buf[..written]),
        ))
    }
}

/// New laptop, waiting for the old laptop's lane message 2.
pub struct LaneOpening {
    hs: HandshakeState,
    lane: u32,
    deadline: Instant,
}

impl LaneOpening {
    /// This lane's number.
    pub fn lane(&self) -> u32 {
        self.lane
    }

    /// Handles lane message 2 at time `now`. Returns the lane's encrypted link and lane message
    /// 3, the sealed confirmation the old laptop needs before it uses the lane.
    pub fn receive(
        mut self,
        msg2: &[u8],
        now: Instant,
    ) -> Result<(Transport, Vec<u8>), PairingError> {
        if now > self.deadline {
            return Err(PairingError::Expired);
        }
        let payload = parse_frame(msg2, KIND_LANE_2)?;
        let mut buf = [0u8; 128];
        let read = self
            .hs
            .read_message(payload, &mut buf)
            .map_err(|_| PairingError::HandshakeFailed)?;
        if read != 0 {
            return Err(PairingError::Malformed);
        }
        let (mut transport, _) = finish_handshake(self.hs)?;
        let mut plain = LANE_READY_LABEL.to_vec();
        plain.extend_from_slice(&self.lane.to_be_bytes());
        let written = transport
            .write_message(&plain, &mut buf)
            .map_err(|_| PairingError::HandshakeFailed)?;
        Ok((Transport(transport), frame(KIND_LANE_3, &buf[..written])))
    }
}

/// Old laptop, waiting for the new laptop's lane message 3.
pub struct LaneAccepting {
    transport: TransportState,
    lane: u32,
    deadline: Instant,
}

impl LaneAccepting {
    /// This lane's number.
    pub fn lane(&self) -> u32 {
        self.lane
    }

    /// Handles lane message 3 at time `now`. Only a confirmation sealed under this lane's fresh
    /// keys and naming this lane makes it usable; a copied lane message 1 cannot produce one.
    pub fn confirm(mut self, msg3: &[u8], now: Instant) -> Result<Transport, PairingError> {
        if now > self.deadline {
            return Err(PairingError::Expired);
        }
        let payload = parse_frame(msg3, KIND_LANE_3)?;
        let mut buf = Zeroizing::new(vec![0u8; payload.len()]);
        let read = self
            .transport
            .read_message(payload, &mut buf)
            .map_err(|_| PairingError::HandshakeFailed)?;
        let mut expected = LANE_READY_LABEL.to_vec();
        expected.extend_from_slice(&self.lane.to_be_bytes());
        if buf[..read] != expected[..] {
            return Err(PairingError::HandshakeFailed);
        }
        Ok(Transport(self.transport))
    }
}

/// Encrypted, tamper-evident link. Messages must be opened in the order they were sealed.
pub struct Transport(TransportState);

impl Transport {
    /// Largest plaintext that fits in one sealed message.
    pub const MAX_PLAINTEXT: usize = NOISE_MAX - AEAD_TAG_LEN;

    /// Encrypts and authenticates one message.
    pub fn seal(&mut self, plaintext: &[u8]) -> Result<Vec<u8>, PairingError> {
        if plaintext.len() > Self::MAX_PLAINTEXT {
            return Err(PairingError::TooLarge);
        }
        let mut out = vec![0u8; plaintext.len() + AEAD_TAG_LEN];
        let written = self
            .0
            .write_message(plaintext, &mut out)
            .map_err(|_| PairingError::Transport)?;
        out.truncate(written);
        Ok(out)
    }

    /// Decrypts one message, rejecting anything altered, replayed out of order or forged.
    /// The plaintext is wiped from memory when the returned buffer is dropped.
    pub fn open(&mut self, sealed: &[u8]) -> Result<Zeroizing<Vec<u8>>, PairingError> {
        if sealed.len() > NOISE_MAX {
            return Err(PairingError::TooLarge);
        }
        let mut out = Zeroizing::new(vec![0u8; sealed.len()]);
        let read = self
            .0
            .read_message(sealed, &mut out)
            .map_err(|_| PairingError::Transport)?;
        out.truncate(read);
        Ok(out)
    }
}

/// What the sender should show right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SenderStatus {
    /// Show the code and a countdown to `code_expires_at`.
    Showing { code_expires_at: Instant },
    /// A failed attempt; no code is shown until `until`.
    CoolingDown { until: Instant },
    /// Too many failed attempts; the person on the sender must choose to start again.
    Locked,
}

/// The sender's changing code: a new code every [`CODE_LIFETIME`], the previous code still
/// accepted for [`ROTATION_GRACE`], exactly one attempt per code, and a failure budget.
pub struct RotatingSender {
    current: SenderEntry,
    previous: Option<SenderEntry>,
    failures: u32,
    blocked_until: Option<Instant>,
    locked: bool,
}

struct SenderEntry {
    code: PairingCode,
    session: SenderSession,
    msg1: Vec<u8>,
    started: Instant,
}

impl SenderEntry {
    fn new(now: Instant, parity: u8) -> Result<Self, PairingError> {
        let code = PairingCode::generate_with_parity(parity)?;
        let (session, msg1) = SenderSession::start(&code, now);
        Ok(Self {
            code,
            session,
            msg1,
            started: now,
        })
    }
}

impl RotatingSender {
    /// Creates the first code at time `now`.
    pub fn new(now: Instant) -> Result<Self, PairingError> {
        Ok(Self {
            current: SenderEntry::new(now, 0)?,
            previous: None,
            failures: 0,
            blocked_until: None,
            locked: false,
        })
    }

    /// A fresh entry to replace the current code: the opposite even/odd of any code still in its
    /// grace period, so the two live codes can always be told apart.
    fn replacement(&self, now: Instant) -> Result<SenderEntry, PairingError> {
        let other = self.previous.as_ref().unwrap_or(&self.current);
        SenderEntry::new(now, 1 - other.code.parity())
    }

    /// What the sender should show right now (call [`tick`](Self::tick) first).
    pub fn status(&self) -> SenderStatus {
        if self.locked {
            SenderStatus::Locked
        } else if let Some(until) = self.blocked_until {
            SenderStatus::CoolingDown { until }
        } else {
            SenderStatus::Showing {
                code_expires_at: self.current.started + CODE_LIFETIME,
            }
        }
    }

    /// The code to show, or `None` while cooling down or locked.
    pub fn code(&self) -> Option<String> {
        self.is_open().then(|| self.current.code.digits())
    }

    /// Message 1 for the code shown now, or `None` while cooling down or locked.
    pub fn message_1(&self) -> Option<&[u8]> {
        self.is_open().then_some(self.current.msg1.as_slice())
    }

    /// Message 1 for the live code with this even/odd last digit: the code shown now, or the one
    /// replaced within the grace period. `None` if there is no such code, or while cooling down or
    /// locked. Call [`tick`](Self::tick) first.
    pub fn message_1_for(&self, parity: u8) -> Option<&[u8]> {
        if !self.is_open() {
            return None;
        }
        [Some(&self.current), self.previous.as_ref()]
            .into_iter()
            .flatten()
            .find(|e| e.code.parity() == parity)
            .map(|e| e.msg1.as_slice())
    }

    /// When the code shown now will be replaced, for the on-screen countdown.
    pub fn expires_at(&self) -> Instant {
        self.current.started + CODE_LIFETIME
    }

    /// Failed attempts in a row so far.
    pub fn failed_attempts(&self) -> u32 {
        self.failures
    }

    /// Advances to time `now`: ends a finished cool-down with a fresh code, replaces the code
    /// when its minute is up, and forgets a previous code once its grace period ends.
    /// Returns `true` if a new code is now showing.
    pub fn tick(&mut self, now: Instant) -> Result<bool, PairingError> {
        if self.locked {
            return Ok(false);
        }
        let mut rotated = false;
        if let Some(until) = self.blocked_until {
            if now < until {
                return Ok(false);
            }
            self.blocked_until = None;
            // Keep the even/odd of the code chosen when the attempt failed (never shown): it
            // already differs from any code in its grace period and from the spent code.
            self.current = SenderEntry::new(now, self.current.code.parity())?;
            rotated = true;
        }
        if now >= self.current.started + CODE_LIFETIME {
            let fresh = SenderEntry::new(now, 1 - self.current.code.parity())?;
            let old = std::mem::replace(&mut self.current, fresh);
            self.previous = Some(old);
            rotated = true;
        }
        if self
            .previous
            .as_ref()
            .is_some_and(|p| now >= p.started + CODE_LIFETIME + ROTATION_GRACE)
        {
            self.previous = None;
        }
        Ok(rotated)
    }

    /// Starts again after a lockout. Only call this when the person on the sender asks to.
    pub fn unlock(&mut self, now: Instant) -> Result<(), PairingError> {
        self.current = SenderEntry::new(now, 1 - self.current.code.parity())?;
        self.previous = None;
        self.failures = 0;
        self.blocked_until = None;
        self.locked = false;
        Ok(())
    }

    /// Handles message 2 at time `now`. Replies are refused without using anything up while
    /// cooling down or locked, and when they are malformed or name an unknown code. A reply for a
    /// known code uses up that code's single attempt; a failure counts against the budget.
    pub fn receive(
        &mut self,
        msg2: &[u8],
        now: Instant,
    ) -> Result<(SenderAwaitingReveal, Vec<u8>), PairingError> {
        self.tick(now)?;
        if self.locked {
            return Err(PairingError::Locked);
        }
        if self.blocked_until.is_some() {
            return Err(PairingError::CoolingDown);
        }
        let id = peek_session_id(msg2)?;
        let used = if id == self.current.session.id {
            let fresh = self.replacement(now)?;
            std::mem::replace(&mut self.current, fresh)
        } else {
            match self.previous.take() {
                Some(p) if id == p.session.id => p,
                other => {
                    self.previous = other;
                    return Err(PairingError::UnknownSession);
                }
            }
        };
        match used.session.receive(msg2, now) {
            Ok(done) => {
                self.failures = 0;
                Ok(done)
            }
            Err(e) => {
                self.record_failure(now);
                Err(e)
            }
        }
    }

    fn is_open(&self) -> bool {
        !self.locked && self.blocked_until.is_none()
    }

    fn record_failure(&mut self, now: Instant) {
        self.failures = self.failures.saturating_add(1);
        if self.failures >= MAX_FAILED_ATTEMPTS {
            self.locked = true;
            self.blocked_until = None;
        } else {
            // 1, 2, 4, 8 seconds for failures 1 to 4.
            let pause = Duration::from_secs(1u64 << (self.failures - 1).min(16));
            self.blocked_until = Some(now + pause);
        }
    }
}

fn peek_session_id(msg2: &[u8]) -> Result<[u8; SESSION_ID_LEN], PairingError> {
    let payload = parse_frame(msg2, KIND_SPAKE_B_NOISE_1)?;
    let id = payload
        .get(..SESSION_ID_LEN)
        .ok_or(PairingError::Malformed)?;
    let mut out = [0u8; SESSION_ID_LEN];
    out.copy_from_slice(id);
    Ok(out)
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
redacted_debug!(
    RotatingSender,
    SenderSession,
    SenderAwaitingReveal,
    SenderChoosing,
    ReceiverSession,
    ReceiverAwaitingApproval,
    Paired,
    LaneKeys,
    LaneOpening,
    LaneAccepting,
    Transport
);

fn frame(kind: u8, payload: &[u8]) -> Vec<u8> {
    let mut msg = Vec::with_capacity(payload.len() + 2);
    msg.push(PROTOCOL_VERSION);
    msg.push(kind);
    msg.extend_from_slice(payload);
    msg
}

fn parse_frame(msg: &[u8], expected_kind: u8) -> Result<&[u8], PairingError> {
    if msg.len() < 2 || msg.len() > MAX_MESSAGE_LEN {
        return Err(PairingError::Malformed);
    }
    if msg[0] != PROTOCOL_VERSION {
        return Err(PairingError::UnsupportedVersion(msg[0]));
    }
    if msg[1] != expected_kind {
        return Err(PairingError::Malformed);
    }
    Ok(&msg[2..])
}

/// Rejects SPAKE2 messages whose point is not a valid, prime-order-subgroup Ed25519 point.
/// Honest messages are always in the prime-order subgroup and never small-order.
fn check_spake_point(msg: &[u8]) -> Result<(), PairingError> {
    let encoded: [u8; 32] = msg
        .get(1..SPAKE2_MSG_LEN)
        .and_then(|b| b.try_into().ok())
        .ok_or(PairingError::Malformed)?;
    let point = CompressedEdwardsY(encoded)
        .decompress()
        .ok_or(PairingError::InvalidKey)?;
    if point.is_small_order() || !point.is_torsion_free() {
        return Err(PairingError::InvalidKey);
    }
    Ok(())
}

fn prologue(session_id: &[u8], spake_a: &[u8], spake_b: &[u8]) -> Vec<u8> {
    let mut p = Vec::with_capacity(
        PROLOGUE_LABEL.len() + 1 + session_id.len() + spake_a.len() + spake_b.len(),
    );
    p.extend_from_slice(PROLOGUE_LABEL);
    p.push(PROTOCOL_VERSION);
    p.extend_from_slice(session_id);
    p.extend_from_slice(spake_a);
    p.extend_from_slice(spake_b);
    p
}

fn derive_psk(spake_key: &[u8]) -> Result<Zeroizing<[u8; 32]>, PairingError> {
    let mut psk = Zeroizing::new([0u8; 32]);
    Hkdf::<Sha256>::new(Some(PSK_SALT), spake_key)
        .expand(b"noise psk", psk.as_mut())
        .map_err(|_| PairingError::HandshakeFailed)?;
    Ok(psk)
}

/// The secret extra lanes are made from: the pairing's pre-shared key, bound to this session's
/// final handshake hash so it belongs to exactly this pairing.
fn derive_lane_secret(
    psk: &[u8; 32],
    handshake_hash: &[u8],
) -> Result<Zeroizing<[u8; 32]>, PairingError> {
    let mut info = b"lane secret".to_vec();
    info.extend_from_slice(handshake_hash);
    let mut secret = Zeroizing::new([0u8; 32]);
    Hkdf::<Sha256>::new(Some(LANE_SECRET_SALT), psk)
        .expand(&info, secret.as_mut())
        .map_err(|_| PairingError::HandshakeFailed)?;
    Ok(secret)
}

fn derive_lane_psk(lane_secret: &[u8; 32], lane: u32) -> Result<Zeroizing<[u8; 32]>, PairingError> {
    let mut info = b"lane psk".to_vec();
    info.extend_from_slice(&lane.to_be_bytes());
    let mut psk = Zeroizing::new([0u8; 32]);
    Hkdf::<Sha256>::new(Some(LANE_PSK_SALT), lane_secret)
        .expand(&info, psk.as_mut())
        .map_err(|_| PairingError::HandshakeFailed)?;
    Ok(psk)
}

fn lane_prologue(lane: u32) -> Vec<u8> {
    let mut p = LANE_PROLOGUE_LABEL.to_vec();
    p.push(PROTOCOL_VERSION);
    p.extend_from_slice(&lane.to_be_bytes());
    p
}

fn noise_builder<'a>(psk: &'a [u8; 32], prologue: &'a [u8]) -> Result<Builder<'a>, PairingError> {
    let params = NOISE_PARAMS
        .parse()
        .map_err(|_| PairingError::HandshakeFailed)?;
    Builder::new(params)
        .psk(0, psk)
        .and_then(|b| b.prologue(prologue))
        .map_err(|_| PairingError::HandshakeFailed)
}

fn finish_handshake(hs: HandshakeState) -> Result<(TransportState, Vec<u8>), PairingError> {
    if !hs.is_handshake_finished() {
        return Err(PairingError::HandshakeFailed);
    }
    let handshake_hash = hs.get_handshake_hash().to_vec();
    let transport = hs
        .into_transport_mode()
        .map_err(|_| PairingError::HandshakeFailed)?;
    Ok((transport, handshake_hash))
}

/// Hash commitment to the receiver's random value (the same idea as Bluetooth numeric comparison
/// and ZRTP): binding because SHA-256 resists collisions, hiding because the value is 32 random bytes.
fn commit_to(nonce: &[u8]) -> [u8; COMMIT_LEN] {
    let mut h = Sha256::new();
    h.update(COMMIT_LABEL);
    h.update(nonce);
    h.finalize().into()
}

fn match_number(handshake_hash: &[u8], nonce: &[u8]) -> Result<u8, PairingError> {
    let mut ikm = Zeroizing::new(Vec::with_capacity(handshake_hash.len() + nonce.len()));
    ikm.extend_from_slice(handshake_hash);
    ikm.extend_from_slice(nonce);
    let mut out = [0u8; 8];
    Hkdf::<Sha256>::new(Some(MATCH_SALT), &ikm)
        .expand(b"match number", &mut out)
        .map_err(|_| PairingError::HandshakeFailed)?;
    Ok(number_in_range(u64::from_le_bytes(out)))
}

fn number_in_range(n: u64) -> u8 {
    let start = u64::from(*MATCH_NUMBER_RANGE.start());
    let span = u64::from(*MATCH_NUMBER_RANGE.end()) - start + 1;
    // The result is at most MATCH_NUMBER_RANGE.end(), which fits in a u8.
    u8::try_from(start + n % span).unwrap_or(*MATCH_NUMBER_RANGE.end())
}

fn random_u64() -> Result<u64, PairingError> {
    let mut b = [0u8; 8];
    getrandom::fill(&mut b).map_err(|_| PairingError::Random)?;
    Ok(u64::from_le_bytes(b))
}

/// The correct number plus two distinct random decoys, shuffled.
fn offer_choices(correct: u8) -> Result<[u8; 3], PairingError> {
    let mut choices = [correct, 0, 0];
    let mut filled = 1;
    let mut draws = 0;
    // Each draw collides with probability at most 2/90, so the cap is only reached if the
    // random source is broken; it keeps this loop finite even then.
    while filled < 3 {
        draws += 1;
        if draws > MAX_CHOICE_DRAWS {
            return Err(PairingError::Random);
        }
        let candidate = number_in_range(random_u64()?);
        if !choices[..filled].contains(&candidate) {
            choices[filled] = candidate;
            filled += 1;
        }
    }
    // Fisher-Yates shuffle so the correct number can sit in any position.
    for i in (1..3).rev() {
        // `i` is 1 or 2, so `i + 1` fits in a u64 and the result fits in a usize.
        let j = usize::try_from(random_u64()? % (i as u64 + 1)).unwrap_or(0);
        choices.swap(i, j);
    }
    Ok(choices)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The match number is only secret because the pre-shared key is mixed in at the very start
    /// of the handshake. If the pattern ever stops being `psk0`, this must fail loudly.
    fn hex(b: &[u8]) -> String {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }

    /// Pinned values (checked against an independent HKDF-SHA256 in Python): the lane secret
    /// binds the handshake hash, and both the lane key and the prologue bind the lane number.
    #[test]
    fn lane_keys_are_pinned() {
        let secret = derive_lane_secret(&[7u8; 32], b"handshake hash").unwrap();
        assert_eq!(
            hex(secret.as_ref()),
            "90c35398ec5ae10d7d7dc2131a8bd9e5b6e6f3212d17b611e20dbc5ef36364e9"
        );
        assert_eq!(
            hex(derive_lane_psk(&[9u8; 32], 3).unwrap().as_ref()),
            "aa6569257aa2f9d091fb80e7a189165f1b8dc314d98f99e29fd1111dff15a5f4"
        );
        assert_eq!(
            hex(&lane_prologue(3)),
            "70637477696e2f76312f6c616e652d70726f6c6f6775650100000003"
        );
        let other = derive_lane_secret(&[7u8; 32], b"another hash").unwrap();
        assert_ne!(secret.as_ref(), other.as_ref());
    }

    /// A confirmation sealed under the lane's own keys but naming another lane is refused.
    #[test]
    fn a_confirmation_naming_another_lane_is_refused() {
        let code = PairingCode::generate().unwrap();
        let now = Instant::now();
        let (sender, msg1) = SenderSession::start(&code, now);
        let (receiver, msg2) = ReceiverSession::respond(&code, &msg1, now).unwrap();
        let (sender_waiting, msg3) = sender.receive(&msg2, now).unwrap();
        let (waiting, msg4) = receiver.receive(&msg3, now).unwrap();
        let choosing = sender_waiting.receive_reveal(&msg4, now).unwrap();
        let (mut old, msg5) = choosing.choose(waiting.match_number(), now).unwrap();
        let mut new = waiting.receive_approval(&msg5, now).unwrap();

        let (mut opening, l1) = new.open_lane(now).unwrap();
        let (accepting, l2) = old.accept_lane(&l1, now).unwrap();
        let mut buf = [0u8; 128];
        opening
            .hs
            .read_message(parse_frame(&l2, KIND_LANE_2).unwrap(), &mut buf)
            .unwrap();
        let mut transport = opening.hs.into_transport_mode().unwrap();
        let mut wrong = LANE_READY_LABEL.to_vec();
        wrong.extend_from_slice(&2u32.to_be_bytes());
        let n = transport.write_message(&wrong, &mut buf).unwrap();
        assert_eq!(
            accepting
                .confirm(&frame(KIND_LANE_3, &buf[..n]), now)
                .map(|_| ())
                .unwrap_err(),
            PairingError::HandshakeFailed
        );
    }

    #[test]
    fn noise_pattern_mixes_the_psk_first() {
        assert_eq!(NOISE_PARAMS, "Noise_NNpsk0_25519_ChaChaPoly_BLAKE2s");
    }

    #[test]
    fn a_locked_sender_refuses_everything_even_a_grace_code_with_the_right_digits() {
        let t0 = Instant::now();
        let mut s = RotatingSender::new(t0).expect("sender");
        let old_code = s.code().expect("code");
        let old_msg1 = s.message_1().expect("msg1").to_vec();
        // The old code moves into its grace period.
        s.tick(t0 + CODE_LIFETIME).expect("tick");
        // Lock while the old code is still within grace.
        let t = t0 + CODE_LIFETIME + Duration::from_secs(1);
        for _ in 0..MAX_FAILED_ATTEMPTS {
            s.record_failure(t);
        }
        assert_eq!(s.status(), SenderStatus::Locked);
        let failures = s.failed_attempts();

        let typed = PairingCode::parse(&old_code).expect("parse");
        let (_r, msg2) =
            ReceiverSession::respond(&typed, &old_msg1, Instant::now()).expect("respond");
        assert!(matches!(s.receive(&msg2, t), Err(PairingError::Locked)));
        assert_eq!(
            s.failed_attempts(),
            failures,
            "a refused reply must not count"
        );
        // Long after, the lock still holds and the old code still cannot pair.
        let later = t + Duration::from_secs(3600);
        assert!(matches!(s.receive(&msg2, later), Err(PairingError::Locked)));
    }

    #[test]
    fn backoff_doubles_then_locks() {
        let now = Instant::now();
        let mut s = RotatingSender::new(now).expect("sender");
        let mut pauses = Vec::new();
        for _ in 0..MAX_FAILED_ATTEMPTS {
            s.record_failure(now);
            pauses.push(s.status());
        }
        assert_eq!(
            pauses,
            vec![
                SenderStatus::CoolingDown {
                    until: now + Duration::from_secs(1)
                },
                SenderStatus::CoolingDown {
                    until: now + Duration::from_secs(2)
                },
                SenderStatus::CoolingDown {
                    until: now + Duration::from_secs(4)
                },
                SenderStatus::CoolingDown {
                    until: now + Duration::from_secs(8)
                },
                SenderStatus::Locked,
            ]
        );
    }
}
