//! The swap inside the first sealed pairing (Security Design Part T): each laptop makes a new key
//! pair for this connection and sends only its public half, with a random value, inside the
//! sealed link. Both work out the same pair secret from the two random values and the two public
//! keys with HKDF (SHA-256), in an order both agree on (the lower key first), so neither role
//! matters. Nothing is kept until the person chooses "Remember this connection".

use hkdf::Hkdf;
use pctwin_pairing::known::{StaticKeys, check_public};
use sha2::Sha256;
use zeroize::Zeroizing;

/// The swap message's form.
const SWAP_VERSION: u8 = 1;
/// `[version][public key, 32][random value, 32]`.
pub const SWAP_LEN: usize = 1 + 32 + 32;
const PAIR_SALT: &[u8] = b"pctwin/v1/known-pair-secret";

/// The other laptop's swap message was not in the expected form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SwapError {
    #[error("the other laptop's key was not in the expected form")]
    Malformed,
    #[error("the system's secure random source failed")]
    Random,
}

/// This laptop's half of a swap, before the other's arrived.
pub struct Swap {
    keys: StaticKeys,
    nonce: Zeroizing<[u8; 32]>,
}

/// A swap both halves of which arrived: what "Remember this connection" keeps. Wiped from memory
/// when dropped.
pub struct Pending {
    pub(crate) keys: StaticKeys,
    pub(crate) peer_public: [u8; 32],
    pub(crate) pair_secret: Zeroizing<[u8; 32]>,
}

impl std::fmt::Debug for Pending {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Pending(..)")
    }
}

impl Pending {
    /// The other laptop's public key for this connection.
    pub fn peer_public(&self) -> [u8; 32] {
        self.peer_public
    }

    /// This laptop's public key for this connection.
    pub fn own_public(&self) -> [u8; 32] {
        self.keys.public()
    }
}

impl Swap {
    /// A new key pair and random value from the system's random source.
    pub fn new() -> Result<Self, SwapError> {
        let keys = StaticKeys::generate().map_err(|_| SwapError::Random)?;
        let mut nonce = Zeroizing::new([0u8; 32]);
        getrandom::fill(nonce.as_mut()).map_err(|_| SwapError::Random)?;
        Ok(Self { keys, nonce })
    }

    /// The message to send inside the sealed link.
    pub fn message(&self) -> Vec<u8> {
        let mut m = Vec::with_capacity(SWAP_LEN);
        m.push(SWAP_VERSION);
        m.extend_from_slice(&self.keys.public());
        m.extend_from_slice(self.nonce.as_ref());
        m
    }

    /// With the other laptop's message `theirs`: both halves, and the pair secret. Refused when
    /// it is not in the exact form, carries a key that makes exchanges predictable, or is this
    /// laptop's own message sent back.
    pub fn finish(self, theirs: &[u8]) -> Result<Pending, SwapError> {
        if theirs.len() != SWAP_LEN || theirs[0] != SWAP_VERSION {
            return Err(SwapError::Malformed);
        }
        let mut peer_public = [0u8; 32];
        peer_public.copy_from_slice(&theirs[1..33]);
        let peer_nonce = &theirs[33..];
        let own = self.keys.public();
        if peer_public == own || peer_nonce == self.nonce.as_slice() {
            return Err(SwapError::Malformed);
        }
        check_public(&peer_public).map_err(|_| SwapError::Malformed)?;
        let (low, high, low_nonce, high_nonce) = if own < peer_public {
            (own, peer_public, self.nonce.as_slice(), peer_nonce)
        } else {
            (peer_public, own, peer_nonce, self.nonce.as_slice())
        };
        let mut ikm = Zeroizing::new(Vec::with_capacity(64));
        ikm.extend_from_slice(low_nonce);
        ikm.extend_from_slice(high_nonce);
        let mut info = Vec::with_capacity(64);
        info.extend_from_slice(&low);
        info.extend_from_slice(&high);
        let mut pair_secret = Zeroizing::new([0u8; 32]);
        Hkdf::<Sha256>::new(Some(PAIR_SALT), &ikm)
            .expand(&info, pair_secret.as_mut())
            .map_err(|_| SwapError::Malformed)?;
        Ok(Pending {
            keys: self.keys,
            peer_public,
            pair_secret,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_laptops_work_out_the_same_pair_secret_and_each_other_s_key() {
        let (a, b) = (Swap::new().unwrap(), Swap::new().unwrap());
        let (ma, mb) = (a.message(), b.message());
        let (pa, pb) = (a.finish(&mb).unwrap(), b.finish(&ma).unwrap());
        assert_eq!(pa.pair_secret.as_ref(), pb.pair_secret.as_ref());
        assert_eq!(pa.peer_public, pb.keys.public());
        assert_eq!(pb.peer_public, pa.keys.public());
        // A new swap: a new secret.
        let (c, d) = (Swap::new().unwrap(), Swap::new().unwrap());
        let pc = c.finish(&d.message()).unwrap();
        assert_ne!(pc.pair_secret.as_ref(), pa.pair_secret.as_ref());
    }

    #[test]
    fn a_message_not_in_its_exact_form_or_sent_back_is_refused() {
        // Its own message sent back.
        let a = Swap::new().unwrap();
        let mine = a.message();
        assert_eq!(a.finish(&mine).err(), Some(SwapError::Malformed));
        // Its own random value with another key.
        let a = Swap::new().unwrap();
        let mut same_nonce = Swap::new().unwrap().message();
        same_nonce[33..].copy_from_slice(&a.message()[33..]);
        assert_eq!(a.finish(&same_nonce).err(), Some(SwapError::Malformed));
        let mut wrong_version = Swap::new().unwrap().message();
        wrong_version[0] = 2;
        let mut low_order = Swap::new().unwrap().message();
        low_order[1..33].copy_from_slice(&[0; 32]);
        let mut longer = Swap::new().unwrap().message();
        longer.push(0);
        for bad in [
            wrong_version,
            low_order,
            longer,
            vec![],
            vec![1; SWAP_LEN - 1],
        ] {
            let a = Swap::new().unwrap();
            assert_eq!(a.finish(&bad).err(), Some(SwapError::Malformed), "{bad:?}");
        }
        // The exact form is accepted.
        assert!(
            Swap::new()
                .unwrap()
                .finish(&Swap::new().unwrap().message())
                .is_ok()
        );
    }
}
