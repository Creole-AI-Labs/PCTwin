//! PCTwin pairing: turns a short one-time code into an authenticated, encrypted link
//! between two laptops (Security Design, Part A).
//!
//! # How it works
//!
//! 1. The sender (old laptop) shows a six-digit [`PairingCode`]. [`RotatingSender`] replaces it
//!    every minute and after any attempt; the previous code keeps a short grace period. Each code
//!    has a random session tag carried in the messages, so a reply is always matched to the code
//!    it was made from and a stray reply never burns the current code.
//! 2. SPAKE2 (Ed25519 group, asymmetric roles) turns the code into a shared secret that an
//!    eavesdropper cannot use to test guesses offline. A wrong code yields a different secret.
//! 3. A Noise `NNpsk0` handshake builds the encrypted link with fresh ephemeral keys
//!    (forward secrecy). The SPAKE2 secret is mixed in as the pre-shared key, and the protocol
//!    version, both roles and both SPAKE2 messages are bound in as the Noise prologue.
//! 4. Both sides derive a two-digit match number from the final handshake hash. The sender shows
//!    it; the receiver offers three numbers (the right one and two random decoys). The person
//!    picks the one shown on the sender. A wrong pick cancels the pairing.
//! 5. Only after the right pick does the receiver send a sealed confirmation, and only then do
//!    either side get a usable [`Paired`] link.
//!
//! Every message carries the protocol version and a message kind, and is size-limited.
//! Malformed input returns an error; it never panics.
//!
//! # Known limits
//!
//! The `spake2` crate states it has had no independent audit and is probably not constant-time;
//! `snow` has had no formal audit. The one-attempt, two-minute code limits online guessing but
//! does not remove those implementation risks. See the Security Design, "Honest gaps".

use std::fmt;
use std::ops::RangeInclusive;
use std::time::{Duration, Instant};

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
const CONFIRM_PAYLOAD: &[u8] = b"pctwin/v1/confirmed";
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
    #[error("the other laptop uses a different PCTwin version ({0})")]
    UnsupportedVersion(u8),
    #[error("a pairing message was not in the expected form")]
    Malformed,
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
            },
            msg3,
        ))
    }
}

/// Sender side, showing its match number and waiting for the receiver's confirmation.
pub struct SenderAwaitingConfirmation {
    transport: TransportState,
    match_number: u8,
}

impl SenderAwaitingConfirmation {
    /// The number to show on the sender's screen.
    pub fn match_number(&self) -> u8 {
        self.match_number
    }

    /// Handles message 4. Only a genuine confirmation from this session unlocks the link.
    pub fn receive_confirmation(mut self, msg4: &[u8]) -> Result<Paired, PairingError> {
        let payload = parse_frame(msg4, KIND_CONFIRM)?;
        let mut buf = vec![0u8; payload.len()];
        let read = self
            .transport
            .read_message(payload, &mut buf)
            .map_err(|_| PairingError::HandshakeFailed)?;
        if &buf[..read] != CONFIRM_PAYLOAD {
            return Err(PairingError::HandshakeFailed);
        }
        Ok(Paired {
            transport: Transport(self.transport),
        })
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

    /// Handles message 3. Returns the three numbers to offer the person.
    pub fn receive(mut self, msg3: &[u8]) -> Result<ReceiverChoosing, PairingError> {
        let payload = parse_frame(msg3, KIND_NOISE_2)?;
        let mut buf = [0u8; 128];
        let read = self
            .hs
            .read_message(payload, &mut buf)
            .map_err(|_| PairingError::HandshakeFailed)?;
        if read != 0 {
            return Err(PairingError::Malformed);
        }
        let (transport, correct) = finish_handshake(self.hs)?;
        let choices = offer_choices(correct)?;
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

    /// Records the person's pick. The right number unlocks the link and returns message 4 to
    /// send; any other number cancels pairing.
    pub fn choose(mut self, picked: u8) -> Result<(Paired, Vec<u8>), PairingError> {
        if picked != self.correct {
            return Err(PairingError::WrongNumber);
        }
        let mut buf = [0u8; 64];
        let written = self
            .transport
            .write_message(CONFIRM_PAYLOAD, &mut buf)
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
    pub fn open(&mut self, sealed: &[u8]) -> Result<Vec<u8>, PairingError> {
        if sealed.len() > NOISE_MAX {
            return Err(PairingError::TooLarge);
        }
        let mut out = vec![0u8; sealed.len()];
        let read = self
            .0
            .read_message(sealed, &mut out)
            .map_err(|_| PairingError::Transport)?;
        out.truncate(read);
        Ok(out)
    }
}

/// The sender's changing code: a new code every [`CODE_LIFETIME`] and after any attempt, with
/// the previous code still accepted for [`ROTATION_GRACE`]. Each code gets exactly one attempt.
pub struct RotatingSender {
    current: SenderEntry,
    previous: Option<SenderEntry>,
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
        })
    }

    /// The code to show now.
    pub fn code(&self) -> String {
        self.current.code.digits()
    }

    /// Message 1 for the code shown now, to send to a receiver.
    pub fn message_1(&self) -> &[u8] {
        &self.current.msg1
    }

    /// When the code shown now will be replaced, for the on-screen countdown.
    pub fn expires_at(&self) -> Instant {
        self.current.started + CODE_LIFETIME
    }

    /// Advances to time `now`: replaces the code when its minute is up and forgets a previous
    /// code once its grace period ends. Returns `true` if a new code is now showing.
    pub fn tick(&mut self, now: Instant) -> Result<bool, PairingError> {
        let mut rotated = false;
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

    /// Handles message 2 at time `now`. A reply for an unknown or expired code is rejected
    /// without using up the current code. A reply for a known code uses up that code's single
    /// attempt; if it was the current code, a new code is shown straight away.
    pub fn receive(
        &mut self,
        msg2: &[u8],
        now: Instant,
    ) -> Result<(SenderAwaitingConfirmation, Vec<u8>), PairingError> {
        self.tick(now)?;
        let id = peek_session_id(msg2)?;
        if id == self.current.session.id {
            let used = std::mem::replace(&mut self.current, SenderEntry::new(now)?);
            return used.session.receive(msg2, now);
        }
        match self.previous.take() {
            Some(used) if id == used.session.id => used.session.receive(msg2, now),
            other => {
                self.previous = other;
                Err(PairingError::UnknownSession)
            }
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
