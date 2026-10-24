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
/// was written, and `SendAgain`); version 1 was never released, so nothing needs it. Version 3
/// sends the old laptop's list in pages (the scan inventory version 2), so no laptop is too big
/// to send; version 2's one JSON document is kept for a laptop one version behind. Version 4
/// added pausing and stopping told to the other laptop ([`HOLD_SINCE`], [`STOP_SINCE`]); with a
/// laptop on version 2 or 3 a pause holds only the old laptop, and a stop looks to the other like
/// a lost connection.
pub const MOVE_PROTOCOL: Versions = Versions {
    oldest: 2,
    newest: 4,
};

/// The first move protocol version with `Hold` (a pause told to the other laptop).
pub const HOLD_SINCE: u16 = 4;

/// The first move protocol version with `Stopping` (a stop told to the other laptop) and
/// `NotSent` (the old laptop says why it did not send a planned file). Version 4 has not been
/// released, so it carries these with `Hold`.
pub const STOP_SINCE: u16 = 4;

/// The first move protocol version where copies cross once: the new laptop may answer a start
/// with the fingerprint of an identical file it already received in this move, in the same
/// person's things, and make the copy itself; and the old laptop sends a file after all when a
/// skip is not accepted. Version 4 has not been released, so it carries this too.
pub const COPY_SINCE: u16 = 4;

/// The first move protocol version where "already there" is judged by the new laptop itself:
/// it sends a key of its own (never its file's fingerprint), the old laptop answers with its
/// file's fingerprint under that key (`Print`), and the new laptop compares. Nothing either
/// laptop says can be matched against anything outside this move (the Engineering Plan's
/// "keyed per move"). And finished files are told again by the file (`ItemDone`), never by a
/// stream number another start may have reused. Version 4 has not been released.
pub const KEYED_SINCE: u16 = 4;

/// The first move protocol version where the new laptop may set a file down partway while the
/// move runs: a start, end or print for a place whose drive paused the move (Quarantine Design,
/// step 4) is answered with `ResumeFrom`, nothing more of the file is written there, and the old
/// laptop picks it up from that ticket once the move carries on. It goes with `Hold`, which
/// tells the old laptop of the pause. Version 4 has not been released, so it carries this too.
pub const SET_DOWN_SINCE: u16 = 4;

/// The first move protocol version whose list (the scan inventory version 3) carries the
/// person's own essentials in the order they move, so the plan the new laptop shows is the
/// order the old laptop sends in. Only the order crosses, never when anything was used. Version
/// 4 has not been released, so it carries this too.
pub const PERSONAL_SINCE: u16 = 4;

/// Message kind, kept apart from the transfer's own (1 to 15) and the inventory's (32, 33, 35).
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
