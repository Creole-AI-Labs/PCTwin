//! Saved Wi-Fi networks (move protocol 8, [`pctwin_transfer::WIFI_SINCE`]): once the plan is
//! approved and the app data has crossed, the old laptop sends one record of the networks its
//! person ticked, with their passwords, inside the sealed link, and nothing else of them.
//!
//! This module only carries the record's bytes, bounded: a head (`[WIFI, HEAD, total: u32,
//! pages: u32]`, big endian), then exactly that many pages (`[WIFI, PAGE, bytes...]`), each at
//! most [`PAGE_BYTES`], adding up to exactly `total`, at most [`MAX_RECORD`]. Every buffer that
//! holds the record's bytes on either side is wiped from memory when dropped. What the bytes say
//! is read, strictly, by `pctwin-fullmove` (`wifi`); a laptop on an older version sends and
//! expects nothing.

use pctwin_transfer::{Channel, TransferError};
use zeroize::Zeroizing;

/// Message kind, apart from the transfer's (1 to 15), the inventory's (32 to 35), landing (39 to
/// 41), the greeting (42 to 44), the way back (45) and the extras (46 to 48).
const WIFI: u8 = 49;
const HEAD: u8 = 0;
const PAGE: u8 = 1;

/// The most bytes one page carries (one sealed message holds at most 64 KiB).
pub const PAGE_BYTES: usize = 60 * 1024;

/// The most bytes the record may hold in all.
pub const MAX_RECORD: usize = 1024 * 1024;

/// The old laptop's side: the record of the networks its person ticked (empty: none).
pub trait WifiOut: std::fmt::Debug + Sync {
    fn record(&self) -> Zeroizing<Vec<u8>>;
}

/// The new laptop's side: where the record goes. Unread bytes from the other laptop, for the
/// part that adds the networks to check.
pub trait WifiIn {
    fn arrived(&mut self, record: Zeroizing<Vec<u8>>);
    /// The connection was lost before the record was whole: none of it is used.
    fn lost(&mut self) {}
}

fn bad(why: &str) -> TransferError {
    TransferError::Protocol(format!("the old laptop's Wi-Fi networks: {why}"))
}

/// The old laptop: sends `record` (empty: no networks).
pub async fn send(ch: &mut impl Channel, record: &[u8]) -> Result<(), TransferError> {
    if record.len() > MAX_RECORD {
        return Err(bad("too large to send"));
    }
    let total = u32::try_from(record.len()).map_err(|_| bad("too large to send"))?;
    let count = u32::try_from(record.len().div_ceil(PAGE_BYTES)).map_err(|_| bad("too large"))?;
    let mut head = vec![WIFI, HEAD];
    head.extend_from_slice(&total.to_be_bytes());
    head.extend_from_slice(&count.to_be_bytes());
    ch.send(&head)
        .await
        .map_err(|_| TransferError::ConnectionDropped)?;
    for chunk in record.chunks(PAGE_BYTES) {
        let mut msg = Zeroizing::new(Vec::with_capacity(chunk.len() + 2));
        msg.push(WIFI);
        msg.push(PAGE);
        msg.extend_from_slice(chunk);
        ch.send(&msg)
            .await
            .map_err(|_| TransferError::ConnectionDropped)?;
    }
    Ok(())
}

/// The head's total and page count, checked against the bounds.
fn head(msg: &[u8]) -> Result<(usize, usize), TransferError> {
    let [WIFI, HEAD, t0, t1, t2, t3, p0, p1, p2, p3] = *msg else {
        return Err(bad("expected their head"));
    };
    let total = u32::from_be_bytes([t0, t1, t2, t3]) as usize;
    let pages = u32::from_be_bytes([p0, p1, p2, p3]) as usize;
    if total > MAX_RECORD || pages != total.div_ceil(PAGE_BYTES) {
        return Err(bad("a head that does not add up"));
    }
    Ok((total, pages))
}

/// The new laptop: the record exactly as the old laptop sent it. Anything else, a page too long,
/// more or fewer bytes than the head said, or more than [`MAX_RECORD`] is refused.
pub async fn receive(ch: &mut impl Channel) -> Result<Zeroizing<Vec<u8>>, TransferError> {
    let first = ch
        .recv()
        .await
        .map_err(|_| TransferError::ConnectionDropped)?;
    let (total, pages) = head(&first)?;
    // Exactly the room it needs, so the bytes are never copied to a larger buffer and left
    // behind unwiped.
    let mut record = Zeroizing::new(Vec::with_capacity(total));
    for n in 0..pages {
        let msg = Zeroizing::new(
            ch.recv()
                .await
                .map_err(|_| TransferError::ConnectionDropped)?,
        );
        let [WIFI, PAGE, body @ ..] = msg.as_slice() else {
            return Err(bad("expected a page"));
        };
        let want = if n + 1 == pages {
            total - PAGE_BYTES * (pages - 1)
        } else {
            PAGE_BYTES
        };
        if body.len() != want {
            return Err(bad("a page of the wrong length"));
        }
        record.extend_from_slice(body);
    }
    Ok(record)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pctwin_transfer::ChannelError;
    use std::collections::VecDeque;

    #[derive(Default)]
    struct Pipe(VecDeque<Vec<u8>>);

    impl Channel for Pipe {
        async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
            self.0.push_back(data.to_vec());
            Ok(())
        }
        async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
            self.0.pop_front().ok_or(ChannelError)
        }
    }

    fn run<T>(f: impl std::future::Future<Output = T>) -> T {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(f)
    }

    #[test]
    fn the_record_arrives_exactly_as_sent_in_bounded_pages() {
        for len in [
            0,
            1,
            PAGE_BYTES - 1,
            PAGE_BYTES,
            PAGE_BYTES + 1,
            3 * PAGE_BYTES + 7,
        ] {
            let bytes: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
            let mut p = Pipe::default();
            run(send(&mut p, &bytes)).unwrap();
            assert_eq!(p.0.len(), 1 + len.div_ceil(PAGE_BYTES), "{len}");
            assert!(p.0.iter().skip(1).all(|m| m.len() <= PAGE_BYTES + 2));
            let got = run(receive(&mut p)).unwrap();
            assert_eq!(*got, bytes, "{len}");
            assert!(p.0.is_empty());
        }
    }

    #[test]
    fn a_record_past_its_bounds_or_out_of_shape_is_refused() {
        // Too large to send at all.
        let mut p = Pipe::default();
        assert!(run(send(&mut p, &vec![0; MAX_RECORD + 1])).is_err());
        assert!(p.0.is_empty());
        let head_of = |total: u32, pages: u32| {
            let mut h = vec![WIFI, HEAD];
            h.extend_from_slice(&total.to_be_bytes());
            h.extend_from_slice(&pages.to_be_bytes());
            h
        };
        // A head saying more than the most, or pages that do not add up.
        for h in [
            head_of(MAX_RECORD as u32 + 1, (MAX_RECORD / PAGE_BYTES + 1) as u32),
            head_of(10, 2),
            head_of(10, 0),
            vec![WIFI, HEAD, 0],
            vec![46, HEAD, 0, 0, 0, 0, 0, 0, 0, 0],
        ] {
            let mut p = Pipe(VecDeque::from([h.clone()]));
            assert!(run(receive(&mut p)).is_err(), "{h:?}");
        }
        // A page longer or shorter than the head said, or of another kind.
        for page in [
            vec![WIFI, PAGE, 1, 2, 3, 4],
            vec![WIFI, PAGE, 1, 2],
            vec![46, PAGE, 1, 2, 3],
        ] {
            let mut p = Pipe(VecDeque::from([head_of(3, 1), page.clone()]));
            assert!(run(receive(&mut p)).is_err(), "{page:?}");
        }
        // The connection ends part way.
        let mut p = Pipe(VecDeque::from([head_of(3, 1)]));
        assert!(matches!(
            run(receive(&mut p)),
            Err(TransferError::ConnectionDropped)
        ));
        // An empty record is a head with no pages.
        let mut p = Pipe(VecDeque::from([head_of(0, 0)]));
        assert!(run(receive(&mut p)).unwrap().is_empty());
    }
}
