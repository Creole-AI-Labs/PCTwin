//! PCTwin pairing: turns a short one-time code into an authenticated, encrypted link
//! between two laptops (Security Design, Part A).
//!
//! # How it works
//!
//! 1. The sender (old laptop) shows a six-digit [`PairingCode`]. [`RotatingSender`] replaces it
//!    every minute; the previous code keeps a short grace period. Each code carries a random
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
//! 5. Both sides derive a two-digit match number from the final handshake hash. The sender shows
//!    it; the receiver offers three numbers (the right one and two random decoys) and the person
//!    picks the one on the sender's screen.
//! 6. The receiver seals the picked number into its confirmation. The sender accepts only a
//!    confirmation carrying its own number, so skipping the comparison does not work even for
//!    someone who knows the code. Only then does either side get a usable [`Paired`] link.
//!
//! Every message carries the protocol version and a message kind, and is size-limited. Malformed
//! input returns an error instead of panicking. Messages that fail before authentication hand the
//! waiting state back ([`Rejected`]), so junk cannot cancel a pairing in progress.
//!
//! # Known limits
//!
//! - `spake2` states it has had no independent audit and is probably not constant-time; `snow`
//!   has had no formal audit. Neither zeroizes its internal secrets on drop.
//! - Session tags are public, so anyone who sees message 1 can spend that code's single attempt
//!   with a junk reply (and repeated junk replies trigger the lockout). Binding attempts to one
//!   network connection belongs to the transport layer.
//! - Time is supplied by the caller. `Instant` does not count suspend on every platform, so the
//!   app must rotate the code when the laptop wakes from sleep.
//! - Starting a session panics if the operating system's random source fails, as SPAKE2 does.

use std::fmt;
use std::ops::RangeInclusive;
use std::time::{Duration, Instant};

use curve25519_dalek::edwards::CompressedEdwardsY;
use hkdf::Hkdf;
use sha2::Sha256;
use snow::{Builder, HandshakeState, TransportState};
use spake2::{Ed25519Group, Identity, Password, Spake2};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

/// Version of the pairing protocol carried in every message.
pub const PROTOCOL_VERSION: u8 = 1;
/// How long a code is shown before it is replaced.
pub const CODE_LIFETIME: Duration = Duration::from_secs(60);
/// How long a replaced code still works, for someone who was typing it when it changed.
pub const ROTATION_GRACE: Duration = Duration::from_secs(15);
/// How long the sender waits for the receiver's confirmation after the handshake.
pub const CONFIRMATION_TIMEOUT: Duration = Duration::from_secs(180);
/// Failed attempts in a row before pairing locks until the person on the sender starts again.
pub const MAX_FAILED_ATTEMPTS: u32 = 5;
/// Largest pairing message accepted, in bytes.
pub const MAX_MESSAGE_LEN: usize = 512;
/// Range of the match number shown during confirmation.
pub const MATCH_NUMBER_RANGE: RangeInclusive<u8> = 10..=99;

const NOISE_PARAMS: &str = "Noise_NNpsk0_25519_ChaChaPoly_BLAKE2s";
const SPAKE2_MSG_LEN: usize = 33;
const SESSION_ID_LEN: usize = 8;
const ID_SENDER: &[u8] = b"pctwin/v1/sender";
const ID_RECEIVER: &[u8] = b"pctwin/v1/receiver";
const PROLOGUE_LABEL: &[u8] = b"pctwin/v1/pairing-prologue";
const PSK_SALT: &[u8] = b"pctwin/v1/pairing-psk";
const MATCH_SALT: &[u8] = b"pctwin/v1/match-number";
const CONFIRM_LABEL: &[u8] = b"pctwin/v1/confirmed";
const AEAD_TAG_LEN: usize = 16;
const NOISE_MAX: usize = 65_535;

const KIND_SPAKE_A: u8 = 1;
const KIND_SPAKE_B_NOISE_1: u8 = 2;
const KIND_NOISE_2: u8 = 3;
const KIND_CONFIRM: u8 = 4;

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
        const LIMIT: u32 = 4_294_000_000;
        loop {
            let mut bytes = [0u8; 4];
            getrandom::fill(&mut bytes).map_err(|_| PairingError::Random)?;
            let n = u32::from_le_bytes(bytes);
            bytes.zeroize();
            if n < LIMIT {
                let mut value = n % 1_000_000;
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
    /// Returns the sender's waiting state (with its match number) and message 3 to send.
    pub fn receive(
        self,
        msg2: &[u8],
        now: Instant,
    ) -> Result<(SenderAwaitingConfirmation, Vec<u8>), PairingError> {
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
        if read != 0 {
            return Err(PairingError::Malformed);
        }
        let written = hs
            .write_message(&[], &mut buf)
            .map_err(|_| PairingError::HandshakeFailed)?;
        let msg3 = frame(KIND_NOISE_2, &buf[..written]);

        let (transport, match_number) = finish_handshake(hs)?;
        Ok((
            SenderAwaitingConfirmation {
                transport,
                match_number,
                deadline: now + CONFIRMATION_TIMEOUT,
            },
            msg3,
        ))
    }
}

/// Sender side, showing its match number and waiting for the receiver's confirmation.
pub struct SenderAwaitingConfirmation {
    transport: TransportState,
    match_number: u8,
    deadline: Instant,
}

impl SenderAwaitingConfirmation {
    /// The number to show on the sender's screen.
    pub fn match_number(&self) -> u8 {
        self.match_number
    }

    /// When the sender stops waiting for confirmation.
    pub fn deadline(&self) -> Instant {
        self.deadline
    }

    /// Handles message 4 at time `now`. Succeeds only for a genuine confirmation from this session
    /// carrying the sender's own number. Junk, truncated or forged messages hand the waiting state
    /// back; a wrong number or a missed deadline ends the pairing.
    pub fn receive_confirmation(
        mut self,
        msg4: &[u8],
        now: Instant,
    ) -> Result<Paired, Rejected<Self>> {
        if now > self.deadline {
            return Err(Rejected::end(PairingError::Expired));
        }
        let payload = match parse_frame(msg4, KIND_CONFIRM) {
            Ok(p) => p,
            Err(e) => return Err(Rejected::keep(e, self)),
        };
        let mut buf = Zeroizing::new(vec![0u8; payload.len()]);
        // A message that fails authentication does not advance the transport's nonce, so the
        // waiting state stays usable for the genuine confirmation.
        let read = match self.transport.read_message(payload, &mut buf) {
            Ok(n) => n,
            Err(_) => return Err(Rejected::keep(PairingError::HandshakeFailed, self)),
        };
        let plain = &buf[..read];
        match plain.split_last() {
            Some((&picked, label)) if label == CONFIRM_LABEL => {
                if picked == self.match_number {
                    Ok(Paired {
                        transport: Transport(self.transport),
                    })
                } else {
                    Err(Rejected::end(PairingError::WrongNumber))
                }
            }
            _ => Err(Rejected::end(PairingError::HandshakeFailed)),
        }
    }
}

/// Receiver side, after the person typed the code and message 2 was sent.
pub struct ReceiverSession {
    hs: HandshakeState,
}

impl ReceiverSession {
    /// Handles message 1 using the typed code. Returns the session and message 2 to send.
    pub fn respond(code: &PairingCode, msg1: &[u8]) -> Result<(Self, Vec<u8>), PairingError> {
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

        let mut buf = [0u8; 128];
        let written = hs
            .write_message(&[], &mut buf)
            .map_err(|_| PairingError::HandshakeFailed)?;
        let mut payload = id.to_vec();
        payload.extend_from_slice(&spake_b);
        payload.extend_from_slice(&buf[..written]);
        Ok((Self { hs }, frame(KIND_SPAKE_B_NOISE_1, &payload)))
    }

    /// Handles message 3. Returns the three numbers to offer the person. Any message that fails
    /// before the handshake completes (junk, truncated, forged) hands the session back: `snow`
    /// restores its handshake state on a failed read, so the genuine message 3 still works.
    pub fn receive(mut self, msg3: &[u8]) -> Result<ReceiverChoosing, Rejected<Self>> {
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
        let (transport, correct) = finish_handshake(self.hs).map_err(Rejected::end)?;
        let choices = offer_choices(correct).map_err(Rejected::end)?;
        Ok(ReceiverChoosing {
            transport,
            correct,
            choices,
        })
    }
}

/// Receiver side, showing three numbers for the person to choose from.
pub struct ReceiverChoosing {
    transport: TransportState,
    correct: u8,
    choices: [u8; 3],
}

impl ReceiverChoosing {
    /// The three numbers to show, in display order. Exactly one matches the sender.
    pub fn choices(&self) -> [u8; 3] {
        self.choices
    }

    /// Records the person's pick. The right number unlocks the link and returns message 4, which
    /// carries the pick sealed for the sender to check. Any other number cancels pairing.
    pub fn choose(mut self, picked: u8) -> Result<(Paired, Vec<u8>), PairingError> {
        if picked != self.correct {
            return Err(PairingError::WrongNumber);
        }
        let mut plain = CONFIRM_LABEL.to_vec();
        plain.push(picked);
        let mut buf = [0u8; 64];
        let written = self
            .transport
            .write_message(&plain, &mut buf)
            .map_err(|_| PairingError::HandshakeFailed)?;
        let msg4 = frame(KIND_CONFIRM, &buf[..written]);
        Ok((
            Paired {
                transport: Transport(self.transport),
            },
            msg4,
        ))
    }
}

/// A confirmed pairing with its encrypted link.
pub struct Paired {
    transport: Transport,
}

impl Paired {
    /// The encrypted link to the other laptop.
    pub fn transport_mut(&mut self) -> &mut Transport {
        &mut self.transport
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
    fn new(now: Instant) -> Result<Self, PairingError> {
        let code = PairingCode::generate()?;
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
            current: SenderEntry::new(now)?,
            previous: None,
            failures: 0,
            blocked_until: None,
            locked: false,
        })
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
            self.current = SenderEntry::new(now)?;
            rotated = true;
        }
        if now >= self.current.started + CODE_LIFETIME {
            let old = std::mem::replace(&mut self.current, SenderEntry::new(now)?);
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
        self.current = SenderEntry::new(now)?;
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
    ) -> Result<(SenderAwaitingConfirmation, Vec<u8>), PairingError> {
        self.tick(now)?;
        if self.locked {
            return Err(PairingError::Locked);
        }
        if self.blocked_until.is_some() {
            return Err(PairingError::CoolingDown);
        }
        let id = peek_session_id(msg2)?;
        let used = if id == self.current.session.id {
            std::mem::replace(&mut self.current, SenderEntry::new(now)?)
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
    SenderAwaitingConfirmation,
    ReceiverSession,
    ReceiverChoosing,
    Paired,
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

fn noise_builder<'a>(psk: &'a [u8; 32], prologue: &'a [u8]) -> Result<Builder<'a>, PairingError> {
    let params = NOISE_PARAMS
        .parse()
        .map_err(|_| PairingError::HandshakeFailed)?;
    Builder::new(params)
        .psk(0, psk)
        .and_then(|b| b.prologue(prologue))
        .map_err(|_| PairingError::HandshakeFailed)
}

fn finish_handshake(hs: HandshakeState) -> Result<(TransportState, u8), PairingError> {
    if !hs.is_handshake_finished() {
        return Err(PairingError::HandshakeFailed);
    }
    let number = match_number(hs.get_handshake_hash())?;
    let transport = hs
        .into_transport_mode()
        .map_err(|_| PairingError::HandshakeFailed)?;
    Ok((transport, number))
}

fn match_number(handshake_hash: &[u8]) -> Result<u8, PairingError> {
    let mut out = [0u8; 8];
    Hkdf::<Sha256>::new(Some(MATCH_SALT), handshake_hash)
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
    while filled < 3 {
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
        let (_r, msg2) = ReceiverSession::respond(&typed, &old_msg1).expect("respond");
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
