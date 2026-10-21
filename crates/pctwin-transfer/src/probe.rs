//! The link test after pairing: the new laptop asks, the old laptop answers with
//! [`PROBE_BYTES`] of filler, and the new laptop times it. That measured speed, never a guess,
//! gives the time the move takes before anything moves (see [`TimeRange`](crate::TimeRange));
//! the move meter takes over once the move runs. Only on move protocol [`PROBE_SINCE`] and
//! later: with an older laptop no time is given.
//!
//! The old laptop could answer faster or slower than the link really is (send early, or hold
//! back); that only changes an estimate the person sees, never what moves.

use std::time::Instant;

use crate::{Channel, TransferError};

/// The first move protocol version with the link test. Version 4 has not been released, so it
/// carries it with `Hold`, `Stopping` and `NotSent`.
pub const PROBE_SINCE: u16 = 4;

/// Message kinds, apart from the transfer's own, the inventory's (32, 33, 35) and the version's
/// (34).
const ASK: u8 = 36;
const FILLER: u8 = 37;

/// Each filler message's bytes after its kind (the size of a block's piece, which the link
/// carries in one message), and how many make one test.
const PIECE: usize = crate::PIECE_MAX;
const PIECES: usize = 64;

/// What one link test moves: 3.75 MiB, a second or two on ordinary Wi-Fi.
pub const PROBE_BYTES: u64 = (PIECE * PIECES) as u64;

/// New laptop: asks for the link test and times it. `None`: the other laptop's version has no
/// link test.
pub async fn measure_link(
    ch: &mut impl Channel,
    version: u16,
) -> Result<Option<u64>, TransferError> {
    if version < PROBE_SINCE {
        return Ok(None);
    }
    let started = Instant::now();
    ch.send(&[ASK])
        .await
        .map_err(|_| TransferError::ConnectionDropped)?;
    for _ in 0..PIECES {
        let got = tokio::time::timeout(crate::VERSION_WAIT, ch.recv())
            .await
            .map_err(|_| TransferError::NoAnswer("it did not answer the link test".into()))?
            .map_err(|_| TransferError::ConnectionDropped)?;
        if got.len() != 1 + PIECE || got[0] != FILLER {
            return Err(TransferError::Protocol(
                "a link test that is not one".into(),
            ));
        }
    }
    let micros = u64::try_from(started.elapsed().as_micros())
        .unwrap_or(u64::MAX)
        .max(1);
    Ok(Some(PROBE_BYTES.saturating_mul(1_000_000) / micros))
}

/// Old laptop: answers the new laptop's link test. Nothing to do with a laptop whose version has
/// none.
pub async fn answer_link_test(ch: &mut impl Channel, version: u16) -> Result<(), TransferError> {
    if version < PROBE_SINCE {
        return Ok(());
    }
    let asked = tokio::time::timeout(crate::VERSION_WAIT, ch.recv())
        .await
        .map_err(|_| TransferError::NoAnswer("it did not ask for the link test".into()))?
        .map_err(|_| TransferError::ConnectionDropped)?;
    if asked != [ASK] {
        return Err(TransferError::Protocol(
            "the new laptop did not ask for the link test".into(),
        ));
    }
    let mut filler = vec![0u8; 1 + PIECE];
    filler[0] = FILLER;
    for _ in 0..PIECES {
        ch.send(&filler)
            .await
            .map_err(|_| TransferError::ConnectionDropped)?;
    }
    Ok(())
}
