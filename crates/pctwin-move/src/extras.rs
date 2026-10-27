//! The full move's extras (move protocol 6, [`pctwin_transfer::EXTRAS_SINCE`]): what the old
//! laptop found beside the files that the new laptop rebuilds (settings found, the old laptop's
//! own folders for pointing paths at the moved files, the special categories), sent once after
//! the list of people, before the person approves anything.
//!
//! This module only carries bytes, bounded: a head (`[EXTRAS, HEAD, total: u32, pages: u32]`, big
//! endian) and then exactly that many pages (`[EXTRAS, PAGE, bytes...]`), each at most
//! [`PAGE_BYTES`], adding up to exactly `total`, at most [`MAX_EXTRAS`]. What the bytes say is
//! read, strictly, by whoever rebuilds them (`pctwin-fullmove`); a laptop on an older version
//! sends and expects nothing.

use pctwin_transfer::{Channel, TransferError};

/// Message kind, apart from the transfer's (1 to 15), the inventory's (32 to 35), landing (39 to
/// 41), the greeting (42 to 44) and the way back (45).
const EXTRAS: u8 = 46;
const HEAD: u8 = 0;
const PAGE: u8 = 1;

/// The most bytes one page carries (one sealed message holds at most 64 KiB).
pub const PAGE_BYTES: usize = 60 * 1024;

/// The most bytes the extras may hold in all.
pub const MAX_EXTRAS: usize = 32 * 1024 * 1024;

fn bad(why: &str) -> TransferError {
    TransferError::Protocol(format!("the old laptop's extras: {why}"))
}

/// The old laptop: sends `bytes` (empty: there are none).
pub async fn send(ch: &mut impl Channel, bytes: &[u8]) -> Result<(), TransferError> {
    if bytes.len() > MAX_EXTRAS {
        return Err(bad("too large to send"));
    }
    let pages = bytes.len().div_ceil(PAGE_BYTES);
    let total = u32::try_from(bytes.len()).map_err(|_| bad("too large to send"))?;
    let count = u32::try_from(pages).map_err(|_| bad("too large to send"))?;
    let mut head = vec![EXTRAS, HEAD];
    head.extend_from_slice(&total.to_be_bytes());
    head.extend_from_slice(&count.to_be_bytes());
    ch.send(&head)
        .await
        .map_err(|_| TransferError::ConnectionDropped)?;
    for chunk in bytes.chunks(PAGE_BYTES) {
        let mut msg = Vec::with_capacity(chunk.len() + 2);
        msg.push(EXTRAS);
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
    let [EXTRAS, HEAD, rest @ ..] = msg else {
        return Err(bad("expected their head"));
    };
    let [t0, t1, t2, t3, p0, p1, p2, p3] = rest else {
        return Err(bad("a head of the wrong length"));
    };
    let total = u32::from_be_bytes([*t0, *t1, *t2, *t3]) as usize;
    let pages = u32::from_be_bytes([*p0, *p1, *p2, *p3]) as usize;
    if total > MAX_EXTRAS || pages != total.div_ceil(PAGE_BYTES) {
        return Err(bad("a head that does not add up"));
    }
    Ok((total, pages))
}

/// The new laptop: the extras the old laptop sent, exactly as sent. Anything else, a page too
/// long, more or fewer bytes than the head said, or more than [`MAX_EXTRAS`] is refused.
pub async fn receive(ch: &mut impl Channel) -> Result<Vec<u8>, TransferError> {
    let first = ch
        .recv()
        .await
        .map_err(|_| TransferError::ConnectionDropped)?;
    let (total, pages) = head(&first)?;
    let mut bytes = Vec::with_capacity(total.min(PAGE_BYTES * 4));
    for n in 0..pages {
        let msg = ch
            .recv()
            .await
            .map_err(|_| TransferError::ConnectionDropped)?;
        let [EXTRAS, PAGE, body @ ..] = msg.as_slice() else {
            return Err(bad("expected a page"));
        };
        let last = n + 1 == pages;
        let want = if last {
            total - PAGE_BYTES * (pages - 1)
        } else {
            PAGE_BYTES
        };
        if body.len() != want {
            return Err(bad("a page of the wrong length"));
        }
        bytes.extend_from_slice(body);
    }
    Ok(bytes)
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
    fn extras_arrive_exactly_as_sent_in_bounded_pages() {
        for len in [0, 1, PAGE_BYTES - 1, PAGE_BYTES, PAGE_BYTES + 1, PAGE_BYTES * 3 + 7] {
            let bytes: Vec<u8> = (0..len).map(|n| (n % 251) as u8).collect();
            let mut pipe = Pipe::default();
            run(send(&mut pipe, &bytes)).unwrap();
            assert_eq!(pipe.0.len(), 1 + len.div_ceil(PAGE_BYTES));
            assert!(pipe.0.iter().all(|m| m.len() <= PAGE_BYTES + 2));
            assert_eq!(run(receive(&mut pipe)).unwrap(), bytes, "{len}");
        }
    }

    #[test]
    fn a_head_or_page_that_does_not_add_up_is_refused() {
        let head_of = |total: u32, pages: u32| {
            let mut h = vec![EXTRAS, HEAD];
            h.extend_from_slice(&total.to_be_bytes());
            h.extend_from_slice(&pages.to_be_bytes());
            h
        };
        // Too large in all.
        let mut p = Pipe(VecDeque::from([head_of(u32::MAX, 1)]));
        assert!(run(receive(&mut p)).is_err());
        // A page count that does not fit the total.
        let mut p = Pipe(VecDeque::from([head_of(10, 2)]));
        assert!(run(receive(&mut p)).is_err());
        // A short page.
        let mut p = Pipe(VecDeque::from([head_of(10, 1), vec![EXTRAS, PAGE, 1, 2]]));
        assert!(run(receive(&mut p)).is_err());
        // Another message where a page belongs.
        let mut p = Pipe(VecDeque::from([head_of(2, 1), vec![41, 1, 2]]));
        assert!(run(receive(&mut p)).is_err());
        // Something else where the head belongs.
        let mut p = Pipe(VecDeque::from([vec![EXTRAS, PAGE, 1]]));
        assert!(run(receive(&mut p)).is_err());
        // Too large to send.
        let mut p = Pipe::default();
        assert!(run(send(&mut p, &vec![0; MAX_EXTRAS + 1])).is_err());
        assert!(p.0.is_empty());
    }
}
