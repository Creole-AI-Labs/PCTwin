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

/* ---------- The app data stream (after the approval) ---------- */

/// The kind of the new laptop's choice of app data (which components, which secrets).
const CHOICE: u8 = 47;
/// The kind of the app data stream.
const DATA: u8 = 48;
const DATA_HEAD: u8 = 1;
const DATA_CHUNK: u8 = 2;
const DATA_END_FILE: u8 = 3;
const DATA_DONE: u8 = 4;
/// Still working (capturing on the old laptop): sent at least every [`DATA_KEEPALIVE`], with the
/// progress so far (the reader's own words), so the link is never silent.
const DATA_WAIT: u8 = 5;

/// How often the old laptop says it is still working while nothing else is sent.
pub const DATA_KEEPALIVE: std::time::Duration = std::time::Duration::from_secs(2);
/// How long the new laptop waits in silence before it takes the link for lost.
pub const DATA_SILENCE: std::time::Duration = std::time::Duration::from_secs(30);
/// The longest progress note.
pub const MAX_PROGRESS: usize = 4 * 1024;

/// What the old laptop has next for the stream.
pub enum NextData {
    /// A file: its header and a reader of its bytes.
    File(Vec<u8>, Box<dyn std::io::Read + Send>),
    /// Nothing yet (still capturing): the progress so far.
    Wait(Vec<u8>),
    /// No more.
    Done,
}

/// The longest header of one file in the stream.
pub const MAX_DATA_HEADER: usize = 60 * 1024;
/// The most bytes of app data one move carries.
pub const MAX_DATA: u64 = 16 * 1024 * 1024 * 1024;
/// The most files in the app data stream.
pub const MAX_DATA_FILES: u64 = 2_000_000;

/// The old laptop's side of the app data: given the new laptop's choice, it gives each file to
/// send, one after another: a header (the reader's own words for what follows) and its bytes.
pub trait DataOut {
    /// The new laptop's choice, as it sent it (unread bytes from the other laptop).
    fn chosen(&mut self, choice: Vec<u8>);
    /// What is next, waiting at most `wait` (the capture runs off the link; meanwhile the link
    /// carries progress).
    fn next_within(&mut self, wait: std::time::Duration) -> NextData;
    /// The connection was lost part way: nothing more is captured.
    fn lost(&mut self) {}
}

/// The new laptop's side: what it chose, and where each file's bytes go. A sink that cannot keep
/// a file answers `Ok` and drops it itself (the stream stays in step); an `Err` ends the move.
pub trait DataIn {
    fn choice(&mut self) -> Vec<u8>;
    fn begin(&mut self, header: &[u8]) -> Result<(), String>;
    fn chunk(&mut self, bytes: &[u8]) -> Result<(), String>;
    fn end(&mut self) -> Result<(), String>;
    /// The old laptop's progress note while it captures.
    fn progress(&mut self, _note: &[u8]) {}
    /// The connection was lost part way: what arrived whole stays, the rest did not arrive.
    fn lost(&mut self) {}
}

/// Nothing chosen, nothing kept: an old laptop with no app data sends none, and a new laptop that
/// chose none drops whatever arrives.
pub struct NoData;

impl DataOut for NoData {
    fn chosen(&mut self, _: Vec<u8>) {}
    fn next_within(&mut self, _: std::time::Duration) -> NextData {
        NextData::Done
    }
}

impl DataIn for NoData {
    fn choice(&mut self) -> Vec<u8> {
        Vec::new()
    }
    fn begin(&mut self, _: &[u8]) -> Result<(), String> {
        Ok(())
    }
    fn chunk(&mut self, _: &[u8]) -> Result<(), String> {
        Ok(())
    }
    fn end(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// The new laptop: sends its choice.
pub async fn send_choice(ch: &mut impl Channel, choice: &[u8]) -> Result<(), TransferError> {
    send_kind(ch, CHOICE, choice).await
}

/// The old laptop: the new laptop's choice.
pub async fn receive_choice(ch: &mut impl Channel) -> Result<Vec<u8>, TransferError> {
    receive_kind(ch, CHOICE).await
}

async fn send_kind(ch: &mut impl Channel, kind: u8, bytes: &[u8]) -> Result<(), TransferError> {
    if bytes.len() > MAX_EXTRAS {
        return Err(bad("too large to send"));
    }
    let pages = bytes.len().div_ceil(PAGE_BYTES);
    let total = u32::try_from(bytes.len()).map_err(|_| bad("too large to send"))?;
    let count = u32::try_from(pages).map_err(|_| bad("too large to send"))?;
    let mut head = vec![kind, HEAD];
    head.extend_from_slice(&total.to_be_bytes());
    head.extend_from_slice(&count.to_be_bytes());
    ch.send(&head)
        .await
        .map_err(|_| TransferError::ConnectionDropped)?;
    for chunk in bytes.chunks(PAGE_BYTES) {
        let mut msg = Vec::with_capacity(chunk.len() + 2);
        msg.push(kind);
        msg.push(PAGE);
        msg.extend_from_slice(chunk);
        ch.send(&msg)
            .await
            .map_err(|_| TransferError::ConnectionDropped)?;
    }
    Ok(())
}

async fn receive_kind(ch: &mut impl Channel, kind: u8) -> Result<Vec<u8>, TransferError> {
    let first = ch
        .recv()
        .await
        .map_err(|_| TransferError::ConnectionDropped)?;
    if first.first() != Some(&kind) {
        return Err(bad("expected another message"));
    }
    let mut as_extras = first.clone();
    as_extras[0] = EXTRAS;
    let (total, pages) = head(&as_extras)?;
    let mut bytes = Vec::with_capacity(total.min(PAGE_BYTES * 4));
    for n in 0..pages {
        let msg = ch
            .recv()
            .await
            .map_err(|_| TransferError::ConnectionDropped)?;
        let [k, PAGE, body @ ..] = msg.as_slice() else {
            return Err(bad("expected a page"));
        };
        if *k != kind {
            return Err(bad("expected a page"));
        }
        let want = if n + 1 == pages {
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

/// The old laptop: sends every file `out` gives, then the end. A file that cannot be read part
/// way is ended short; the new laptop's own checks (size and hash) refuse it.
pub async fn send_data(ch: &mut impl Channel, out: &mut dyn DataOut) -> Result<(), TransferError> {
    send_data_every(ch, out, DATA_KEEPALIVE).await
}

/// [`send_data`], saying it is still working at least every `keepalive`.
pub async fn send_data_every(
    ch: &mut impl Channel,
    out: &mut dyn DataOut,
    keepalive: std::time::Duration,
) -> Result<(), TransferError> {
    let mut buf = vec![0u8; PAGE_BYTES];
    loop {
        let (header, mut reader) = match out.next_within(keepalive) {
            NextData::Done => break,
            NextData::Wait(note) => {
                let mut msg = vec![DATA, DATA_WAIT];
                msg.extend_from_slice(&note[..note.len().min(MAX_PROGRESS)]);
                ch.send(&msg)
                    .await
                    .map_err(|_| TransferError::ConnectionDropped)?;
                continue;
            }
            NextData::File(h, r) => (h, r),
        };
        if header.len() > MAX_DATA_HEADER {
            return Err(bad("a header too large"));
        }
        let mut msg = vec![DATA, DATA_HEAD];
        msg.extend_from_slice(&header);
        ch.send(&msg)
            .await
            .map_err(|_| TransferError::ConnectionDropped)?;
        loop {
            let n = match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            };
            let mut msg = Vec::with_capacity(n + 2);
            msg.push(DATA);
            msg.push(DATA_CHUNK);
            msg.extend_from_slice(&buf[..n]);
            ch.send(&msg)
                .await
                .map_err(|_| TransferError::ConnectionDropped)?;
        }
        ch.send(&[DATA, DATA_END_FILE])
            .await
            .map_err(|_| TransferError::ConnectionDropped)?;
    }
    ch.send(&[DATA, DATA_DONE])
        .await
        .map_err(|_| TransferError::ConnectionDropped)
}

/// The new laptop: receives the stream into `sink`, bounded ([`MAX_DATA`], [`MAX_DATA_FILES`]).
pub async fn receive_data(
    ch: &mut impl Channel,
    sink: &mut dyn DataIn,
) -> Result<(), TransferError> {
    receive_data_within(ch, sink, DATA_SILENCE).await
}

/// [`receive_data`], taking the link for lost after `silence` with nothing from the old laptop.
pub async fn receive_data_within(
    ch: &mut impl Channel,
    sink: &mut dyn DataIn,
    silence: std::time::Duration,
) -> Result<(), TransferError> {
    let mut bytes = 0u64;
    let mut files = 0u64;
    let mut open = false;
    loop {
        let msg = tokio::time::timeout(silence, ch.recv())
            .await
            .map_err(|_| TransferError::ConnectionDropped)?
            .map_err(|_| TransferError::ConnectionDropped)?;
        match msg.as_slice() {
            [DATA, DATA_WAIT, note @ ..] => {
                if note.len() > MAX_PROGRESS {
                    return Err(bad("a progress note too long"));
                }
                sink.progress(note);
            }
            [DATA, DATA_HEAD, header @ ..] if !open => {
                files += 1;
                if header.len() > MAX_DATA_HEADER || files > MAX_DATA_FILES {
                    return Err(bad("app data past its limits"));
                }
                sink.begin(header).map_err(|e| bad(&e))?;
                open = true;
            }
            [DATA, DATA_CHUNK, body @ ..] if open => {
                bytes += body.len() as u64;
                if bytes > MAX_DATA || body.len() > PAGE_BYTES {
                    return Err(bad("app data past its limits"));
                }
                sink.chunk(body).map_err(|e| bad(&e))?;
            }
            [DATA, DATA_END_FILE] if open => {
                sink.end().map_err(|e| bad(&e))?;
                open = false;
            }
            [DATA, DATA_DONE] if !open => return Ok(()),
            _ => return Err(bad("app data out of order")),
        }
    }
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
            .enable_all()
            .build()
            .unwrap()
            .block_on(f)
    }

    #[test]
    fn extras_arrive_exactly_as_sent_in_bounded_pages() {
        for len in [
            0,
            1,
            PAGE_BYTES - 1,
            PAGE_BYTES,
            PAGE_BYTES + 1,
            PAGE_BYTES * 3 + 7,
        ] {
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

    struct Out(Vec<(Vec<u8>, Vec<u8>)>, Option<Vec<u8>>);
    impl DataOut for Out {
        fn chosen(&mut self, choice: Vec<u8>) {
            self.1 = Some(choice);
        }
        fn next_within(&mut self, _: std::time::Duration) -> NextData {
            if self.0.is_empty() {
                return NextData::Done;
            }
            let (h, b) = self.0.remove(0);
            NextData::File(h, Box::new(std::io::Cursor::new(b)))
        }
    }

    #[derive(Default)]
    struct In(Vec<(Vec<u8>, Vec<u8>)>);
    impl DataIn for In {
        fn choice(&mut self) -> Vec<u8> {
            b"pick".to_vec()
        }
        fn begin(&mut self, header: &[u8]) -> Result<(), String> {
            self.0.push((header.to_vec(), Vec::new()));
            Ok(())
        }
        fn chunk(&mut self, bytes: &[u8]) -> Result<(), String> {
            self.0
                .last_mut()
                .ok_or("no file")?
                .1
                .extend_from_slice(bytes);
            Ok(())
        }
        fn end(&mut self) -> Result<(), String> {
            Ok(())
        }
    }

    #[test]
    fn app_data_crosses_file_by_file_after_the_choice() {
        let big: Vec<u8> = (0..(PAGE_BYTES * 2 + 5)).map(|n| (n % 7) as u8).collect();
        let files = vec![(b"a".to_vec(), big), (b"b".to_vec(), Vec::new())];
        let mut pipe = Pipe::default();
        let mut sink = In::default();
        run(send_choice(&mut pipe, &sink.choice())).unwrap();
        let mut out = Out(files.clone(), None);
        out.chosen(run(receive_choice(&mut pipe)).unwrap());
        assert_eq!(out.1.as_deref(), Some(&b"pick"[..]));
        run(send_data(&mut pipe, &mut out)).unwrap();
        run(receive_data(&mut pipe, &mut sink)).unwrap();
        assert_eq!(sink.0, files);
    }

    #[test]
    fn app_data_out_of_order_is_refused() {
        let mut p = Pipe(VecDeque::from([vec![DATA, DATA_CHUNK, 1]]));
        assert!(run(receive_data(&mut p, &mut In::default())).is_err());
        let mut p = Pipe(VecDeque::from([
            vec![DATA, DATA_HEAD, 1],
            vec![DATA, DATA_DONE],
        ]));
        assert!(run(receive_data(&mut p, &mut In::default())).is_err());
    }

    /// A capture slower than the new laptop's silence limit: the old laptop is "still working"
    /// for 1.5 s before its file, while the new laptop gives up after 0.3 s of silence.
    struct Slow {
        until: std::time::Instant,
        sent: bool,
    }
    impl DataOut for Slow {
        fn chosen(&mut self, _: Vec<u8>) {}
        fn next_within(&mut self, wait: std::time::Duration) -> NextData {
            if self.sent {
                return NextData::Done;
            }
            let now = std::time::Instant::now();
            if now < self.until {
                std::thread::sleep(wait.min(self.until - now));
                return NextData::Wait(b"{\"done\":0,\"total\":1}".to_vec());
            }
            self.sent = true;
            NextData::File(
                b"h".to_vec(),
                Box::new(std::io::Cursor::new(b"slow bytes".to_vec())),
            )
        }
    }

    struct Tokio {
        tx: tokio::sync::mpsc::UnboundedSender<Vec<u8>>,
        rx: tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>,
    }
    impl Channel for Tokio {
        async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
            self.tx.send(data.to_vec()).map_err(|_| ChannelError)
        }
        async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
            self.rx.recv().await.ok_or(ChannelError)
        }
    }
    fn tokio_pair() -> (Tokio, Tokio) {
        let (a_tx, b_rx) = tokio::sync::mpsc::unbounded_channel();
        let (b_tx, a_rx) = tokio::sync::mpsc::unbounded_channel();
        (Tokio { tx: a_tx, rx: a_rx }, Tokio { tx: b_tx, rx: b_rx })
    }

    #[derive(Default)]
    struct Noted(In, Vec<Vec<u8>>);
    impl DataIn for Noted {
        fn choice(&mut self) -> Vec<u8> {
            Vec::new()
        }
        fn begin(&mut self, h: &[u8]) -> Result<(), String> {
            self.0.begin(h)
        }
        fn chunk(&mut self, b: &[u8]) -> Result<(), String> {
            self.0.chunk(b)
        }
        fn end(&mut self) -> Result<(), String> {
            self.0.end()
        }
        fn progress(&mut self, note: &[u8]) {
            self.1.push(note.to_vec());
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_capture_slower_than_the_silence_limit_keeps_the_link_alive_and_succeeds() {
        use std::time::Duration;
        let (mut old, mut new) = tokio_pair();
        let sender = std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async move {
                    let mut out = Slow {
                        until: std::time::Instant::now() + Duration::from_millis(1500),
                        sent: false,
                    };
                    send_data_every(&mut old, &mut out, Duration::from_millis(100)).await
                })
        });
        let mut sink = Noted::default();
        receive_data_within(&mut new, &mut sink, Duration::from_millis(300))
            .await
            .unwrap();
        sender.join().unwrap().unwrap();
        assert_eq!(sink.0.0, vec![(b"h".to_vec(), b"slow bytes".to_vec())]);
        assert!(sink.1.len() >= 5, "progress kept coming: {}", sink.1.len());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn without_word_from_the_old_laptop_the_link_is_taken_for_lost() {
        use std::time::Duration;
        let (mut old, mut new) = tokio_pair();
        let sender = std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async move {
                    let mut out = Slow {
                        until: std::time::Instant::now() + Duration::from_millis(1200),
                        sent: false,
                    };
                    // Its "still working" comes only every second: past the new laptop's limit.
                    send_data_every(&mut old, &mut out, Duration::from_millis(1000)).await
                })
        });
        let got =
            receive_data_within(&mut new, &mut Noted::default(), Duration::from_millis(300)).await;
        assert!(
            matches!(got, Err(TransferError::ConnectionDropped)),
            "{got:?}"
        );
        drop(new);
        let _ = sender.join();
    }
}
