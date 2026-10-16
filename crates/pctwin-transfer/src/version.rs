//! The move protocol version handshake: right after the number match, before the old laptop's
//! list, both laptops say the oldest and newest move protocol they speak, and both use the highest
//! version they share (Engineering Plan 9.2). It runs inside the sealed link, so nobody in between
//! can change what either laptop offers and talk them down to an older version. Both laptops send
//! their range at once and compute the same answer from the two ranges, so neither one picks for
//! the other.
//!
//! The pairing protocol has its own version (`pctwin_pairing::PROTOCOL_VERSION`), carried in every
//! pairing message and bound into its handshake; this one covers everything sent after pairing.
//!
//! This message's form is fixed for every release, since it is what a laptop on another version
//! reads first after pairing: a kind byte, then the oldest and the newest version as big-endian 16-bit numbers.

use std::time::Duration;

use crate::{Channel, TransferError};

/// How long a laptop waits for the other to say which versions it speaks. Both send at once, so
/// a laptop that is there answers straight away.
pub const VERSION_WAIT: Duration = Duration::from_secs(30);

/// The move protocol versions this release speaks. Each new version keeps the one before it
/// (Engineering Plan 9.2: the current protocol and at least one version back), counting from the
/// first released one. Version 2 added sending damaged blocks again (receipts say whether a block
/// was written, and `SendAgain`); version 1 was never released, so nothing needs it.
pub const MOVE_PROTOCOL: Versions = Versions {
    oldest: 2,
    newest: 2,
};

/// Message kind, kept apart from the transfer's own (1 to 15) and the inventory's (32, 33).
const KIND: u8 = 34;
const LEN: usize = 5;

/// A range of move protocol versions, from 1 up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Versions {
    oldest: u16,
    newest: u16,
}

impl Versions {
    /// The versions from `oldest` to `newest`, or `None` if it starts at 0 or runs backwards.
    pub const fn new(oldest: u16, newest: u16) -> Option<Self> {
        if oldest == 0 || oldest > newest {
            None
        } else {
            Some(Versions { oldest, newest })
        }
    }

    pub const fn oldest(self) -> u16 {
        self.oldest
    }

    pub const fn newest(self) -> u16 {
        self.newest
    }
}

/// Which laptop needs a newer PCTwin when the two share no version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum VersionGap {
    #[error(
        "PCTwin on this laptop is older than on the other laptop; update it here, then start again"
    )]
    ThisLaptopOlder,
    #[error(
        "PCTwin on the other laptop is older than on this one; update it there, then start again"
    )]
    OtherLaptopOlder,
}

/// The highest version in both `ours` and `theirs`, or which laptop is older if they share none.
pub fn choose_version(ours: Versions, theirs: Versions) -> Result<u16, VersionGap> {
    if ours.newest < theirs.oldest {
        Err(VersionGap::ThisLaptopOlder)
    } else if theirs.newest < ours.oldest {
        Err(VersionGap::OtherLaptopOlder)
    } else {
        Ok(ours.newest.min(theirs.newest))
    }
}

/// Sends `ours`, reads the other laptop's range and returns the version both use. Run on both
/// laptops as the first step on the sealed link. A laptop that does not answer within
/// [`VERSION_WAIT`] is given up on.
pub async fn agree_version(ch: &mut impl Channel, ours: Versions) -> Result<u16, TransferError> {
    let mut msg = Vec::with_capacity(LEN);
    msg.push(KIND);
    msg.extend_from_slice(&ours.oldest.to_be_bytes());
    msg.extend_from_slice(&ours.newest.to_be_bytes());
    ch.send(&msg)
        .await
        .map_err(|_| TransferError::ConnectionDropped)?;

    let got = tokio::time::timeout(VERSION_WAIT, ch.recv())
        .await
        .map_err(|_| TransferError::NoAnswer("it did not say which PCTwin it has".into()))?
        .map_err(|_| TransferError::ConnectionDropped)?;
    let theirs = decode(&got).ok_or_else(|| {
        TransferError::Protocol("its PCTwin version could not be read".to_string())
    })?;
    choose_version(ours, theirs).map_err(TransferError::VersionGap)
}

fn decode(msg: &[u8]) -> Option<Versions> {
    let [KIND, a, b, c, d] = *msg else {
        return None;
    };
    Versions::new(u16::from_be_bytes([a, b]), u16::from_be_bytes([c, d]))
}
