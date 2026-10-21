//! The inventory exchange: after the number match, the old laptop sends its scanned record (the
//! inventory) to the new laptop over the sealed link, before any file.
//!
//! The record is compressed and sent in pieces that each fit the link: first a start frame that
//! announces the record's size and its packed size, then the packed bytes. The new laptop treats
//! what arrives as coming from a stranger, since someone holding the code could pose as the old
//! laptop (Security Design A). Every size is checked before anything is held or unpacked: the
//! record must be no larger than [`MAX_INVENTORY_BYTES`], its packed size no larger than
//! compression can make it, the pieces must add up to exactly the packed size, and it must unpack
//! to exactly the announced size (unpacking stops one byte past it). Then it is checked like a
//! saved record. Every piece but the last is full, so the number of pieces is fixed by the
//! announced size.
//!
//! Time: each frame must arrive within [`INVENTORY_PIECE_WAIT`], so a laptop that connects and
//! then says nothing, or stalls part way through a frame, is given up on instead of leaving the
//! new laptop waiting for ever. There is no deadline on the whole list: any pace fast enough for
//! a full piece every 30 seconds (about 16 kbit/s) is a slow link, not a stall, and is never cut
//! off (a minimum pace for the whole list would refuse honest laptops on slow connections). A
//! sender that keeps sending real pieces slowly is shown to the person as slow progress, which
//! they can cancel. Where people and shared things go on the new laptop is decided there ("Who's
//! moving?"), never by the old laptop, so a record that arrives with those choices made is refused,
//! as is one that is not a fresh scan (revisions are made on the new laptop).
//!
//! Memory: receiving holds about 2.9 times the record's JSON size at its peak, and the record
//! itself about 1.7 times (measured with 300,000 items, 14 October 2026), so the limit means about
//! 1.5 GB at the very most: an honest laptop with about 1.8 million files, or a sender that sends
//! the largest record allowed. In pages (move protocol 3), the new laptop holds one page's bytes
//! (at most 8 MB) besides the items read so far, and the reader counts what each page will hold
//! before copying it, refusing the list past `pctwin_record::MAX_HELD` (1.25 GiB), however many
//! pages a sender announces or sends. The count is an upper bound on what the allocator takes;
//! CI measures it on Windows, macOS and Linux (`scripts/inventory-memory.py`).

use std::io::Read;
use std::time::Duration;

use pctwin_record::inventory::MAX_MANIFEST_BYTES;
use pctwin_record::{EncodedInventory, InventoryReader, MAX_PAGE_BYTES, Record, encode_inventory};

use crate::{Channel, PIECE_MAX, TransferError};

/// The largest record accepted, as JSON before compression (about 1.3 million items).
pub const MAX_INVENTORY_BYTES: u64 = 512 * 1024 * 1024;

/// Longest wait for each frame of the list.
pub const INVENTORY_PIECE_WAIT: Duration = Duration::from_secs(30);

/// Message kinds, kept apart from the transfer's own (1 to 15), so neither is mistaken for the
/// other.
const START: u8 = 32;
const PART: u8 = 33;
/// Paged inventory (move protocol 3): the next bytes are the manifest or one page.
const PAGED: u8 = 35;
/// The index a paged header gives the manifest.
const MANIFEST: u32 = u32::MAX;
/// The first move protocol version that sends the inventory in pages.
pub const PAGED_FROM: u16 = 3;

/// Compression level: fast, and still small for the repetitive text of a record.
const LEVEL: i32 = 3;

/// One frame of the inventory exchange.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InventoryFrame {
    /// The record's size as JSON, and its size once compressed.
    Start { raw_len: u64, packed_len: u64 },
    /// The next piece of the compressed record.
    Part(Vec<u8>),
}

impl InventoryFrame {
    pub fn encode(&self) -> Vec<u8> {
        match self {
            InventoryFrame::Start {
                raw_len,
                packed_len,
            } => {
                let mut w = Vec::with_capacity(17);
                w.push(START);
                w.extend_from_slice(&raw_len.to_be_bytes());
                w.extend_from_slice(&packed_len.to_be_bytes());
                w
            }
            InventoryFrame::Part(bytes) => {
                let mut w = Vec::with_capacity(1 + bytes.len());
                w.push(PART);
                w.extend_from_slice(bytes);
                w
            }
        }
    }

    /// Reads one frame. Anything else, cut short or with bytes left over is refused.
    pub fn decode(bytes: &[u8]) -> Result<Self, TransferError> {
        match bytes.split_first() {
            Some((&START, rest)) => {
                let fields: [u8; 16] = rest
                    .try_into()
                    .map_err(|_| refused("its start was damaged"))?;
                let (raw, packed) = fields.split_at(8);
                Ok(InventoryFrame::Start {
                    raw_len: u64::from_be_bytes(raw.try_into().unwrap_or_default()),
                    packed_len: u64::from_be_bytes(packed.try_into().unwrap_or_default()),
                })
            }
            Some((&PART, rest)) => Ok(InventoryFrame::Part(rest.to_vec())),
            _ => Err(refused("something other than the list arrived")),
        }
    }
}

/// Old laptop: sends `record` to the new laptop in the form move protocol `version` uses (pages
/// from [`PAGED_FROM`], one JSON document before). The record carries no choices about where
/// people or shared things go; those are made on the new laptop.
pub async fn send_inventory(
    ch: &mut impl Channel,
    record: &Record,
    version: u16,
) -> Result<(), TransferError> {
    if version >= PAGED_FROM {
        send_paged(ch, record).await
    } else {
        send_within(ch, record, MAX_INVENTORY_BYTES).await
    }
}

/// The scan inventory version 2 (`pctwin_record::inventory`): the manifest, then each page, each
/// announced by its index and length and sent in full pieces but the last.
async fn send_paged(ch: &mut impl Channel, record: &Record) -> Result<(), TransferError> {
    if !undecided(record) || record.revision != 1 || record.previous.is_some() {
        return Err(refused(
            "it already says where people or shared things go, which only the new laptop decides",
        ));
    }
    let EncodedInventory { manifest, pages } =
        encode_inventory(&record.source_laptop, &record.items)
            .map_err(|e| refused(&e.to_string()))?;
    send_blob(ch, MANIFEST, &manifest).await?;
    for (n, page) in pages.iter().enumerate() {
        let n = u32::try_from(n).map_err(|_| refused("it has too many pages"))?;
        send_blob(ch, n, page).await?;
    }
    Ok(())
}

async fn send_blob(ch: &mut impl Channel, index: u32, bytes: &[u8]) -> Result<(), TransferError> {
    let len = u32::try_from(bytes.len()).map_err(|_| refused("a page is too large"))?;
    let mut head = Vec::with_capacity(9);
    head.push(PAGED);
    head.extend_from_slice(&index.to_be_bytes());
    head.extend_from_slice(&len.to_be_bytes());
    send(ch, &head).await?;
    for piece in bytes.chunks(PIECE_MAX) {
        send(ch, &InventoryFrame::Part(piece.to_vec()).encode()).await?;
    }
    Ok(())
}

/// New laptop: the manifest or page `index`, at most `max` bytes, checked piece by piece: the
/// header names exactly this index and a length within `max`, every piece but the last is full,
/// and nothing past the length arrives. It is read into `bytes`, which is reused from page to
/// page, so no page buffer is made and let go each time.
async fn receive_blob(
    ch: &mut impl Channel,
    index: u32,
    max: usize,
    bytes: &mut Vec<u8>,
) -> Result<(), TransferError> {
    let head = recv_in_time(ch).await?;
    let [PAGED, a, b, c, d, e, f, g, h] = *head.as_slice() else {
        return Err(refused("a page did not begin with its size"));
    };
    if u32::from_be_bytes([a, b, c, d]) != index {
        return Err(refused("a page arrived out of order"));
    }
    let len = u32::from_be_bytes([e, f, g, h]) as usize;
    if len == 0 || len > max {
        return Err(refused("a page is empty or larger than PCTwin accepts"));
    }
    // Room for the length, at most a page (already allowed for), and each piece read in place:
    // nothing grows or is copied and let go while a page arrives, so no gaps are left between
    // the blocks the reader keeps.
    bytes.clear();
    bytes.reserve_exact(len);
    while bytes.len() < len {
        let frame = recv_in_time(ch).await?;
        let piece = match frame.split_first() {
            Some((&PART, piece)) => piece,
            Some((&START, _)) => return Err(refused("a page started again part way through")),
            _ => return Err(refused("something other than the list arrived")),
        };
        let left = len - bytes.len();
        if piece.is_empty() || piece.len() > PIECE_MAX || piece.len() > left {
            return Err(refused("a piece of a page was the wrong size"));
        }
        if piece.len() < PIECE_MAX && piece.len() < left {
            return Err(refused("a piece before the last was not full"));
        }
        bytes.extend_from_slice(piece);
    }
    Ok(())
}

/// New laptop, move protocol 3 and later: the paged inventory, read as from a stranger page by
/// page ([`InventoryReader`]), each page refused the moment it breaks a rule.
async fn receive_paged(ch: &mut impl Channel) -> Result<Record, TransferError> {
    // One buffer for the manifest and every page, and one list for every item.
    let mut bytes = Vec::new();
    receive_blob(ch, MANIFEST, MAX_MANIFEST_BYTES, &mut bytes).await?;
    let mut reader = InventoryReader::start(&bytes).map_err(|e| refused(&e.to_string()))?;
    if reader.item_count() == 0 {
        return Err(refused("it was empty"));
    }
    let announced = usize::try_from(reader.item_count()).unwrap_or(usize::MAX);
    let mut items = Vec::with_capacity(announced.min(100_000));
    for n in 0..reader.page_count() {
        receive_blob(ch, n, MAX_PAGE_BYTES, &mut bytes).await?;
        reader
            .page_into(&bytes, &mut items)
            .map_err(|e| refused(&e.to_string()))?;
    }
    drop(bytes);
    let laptop = reader.laptop();
    reader.finish().map_err(|e| refused(&e.to_string()))?;
    let mut record = Record::new(laptop);
    record.items = items;
    record.validate().map_err(|e| refused(&e.to_string()))?;
    Ok(record)
}

/// [`send_inventory`] for a record of at most `limit` bytes as JSON.
async fn send_within(
    ch: &mut impl Channel,
    record: &Record,
    limit: u64,
) -> Result<(), TransferError> {
    if !undecided(record) {
        return Err(refused(
            "it already says where people or shared things go, which only the new laptop decides",
        ));
    }
    let json = record.to_json();
    if json.len() as u64 > limit {
        return Err(refused("it is larger than PCTwin can send in one move"));
    }
    let packed = zstd::bulk::compress(&json, LEVEL)?;
    let start = InventoryFrame::Start {
        raw_len: json.len() as u64,
        packed_len: packed.len() as u64,
    };
    send(ch, &start.encode()).await?;
    for piece in packed.chunks(PIECE_MAX) {
        send(ch, &InventoryFrame::Part(piece.to_vec()).encode()).await?;
    }
    Ok(())
}

/// New laptop: receives the old laptop's record, in the form move protocol `version` uses, and
/// checks it. The person then reviews it and approves a revision of their own; nothing in it is
/// acted on before that.
pub async fn receive_inventory(
    ch: &mut impl Channel,
    version: u16,
) -> Result<Record, TransferError> {
    if version >= PAGED_FROM {
        receive_paged(ch).await
    } else {
        receive_json(ch).await
    }
}

/// Move protocol 2: the record as one compressed JSON document.
async fn receive_json(ch: &mut impl Channel) -> Result<Record, TransferError> {
    let InventoryFrame::Start {
        raw_len,
        packed_len,
    } = InventoryFrame::decode(&recv_in_time(ch).await?)?
    else {
        return Err(refused("it did not begin with its size"));
    };
    if raw_len == 0 {
        return Err(refused("it was empty"));
    }
    if raw_len > MAX_INVENTORY_BYTES {
        return Err(refused("it is larger than PCTwin accepts in one move"));
    }
    // Within the limit, so it fits in memory sizes on every system.
    let raw = usize::try_from(raw_len).map_err(|_| refused("it is too large"))?;
    let packed_len = usize::try_from(packed_len).map_err(|_| refused("it is too large"))?;
    // An empty packed record unpacks to nothing, so it is refused below as the wrong size.
    if packed_len > zstd::zstd_safe::compress_bound(raw) {
        return Err(refused("its sizes do not fit together"));
    }

    let mut packed = Vec::with_capacity(packed_len.min(1024 * 1024));
    while packed.len() < packed_len {
        let InventoryFrame::Part(piece) = InventoryFrame::decode(&recv_in_time(ch).await?)? else {
            return Err(refused("it started again part way through"));
        };
        if piece.is_empty() || piece.len() > PIECE_MAX {
            return Err(refused("a piece of it was the wrong size"));
        }
        if piece.len() > packed_len - packed.len() {
            return Err(refused("more of it arrived than was announced"));
        }
        if piece.len() < PIECE_MAX && piece.len() < packed_len - packed.len() {
            return Err(refused("a piece before the last was not full"));
        }
        packed.extend_from_slice(&piece);
    }

    let json = unpack(&packed, raw_len)?;
    // Only one copy is held while the record is read.
    drop(packed);
    if json.len() != raw {
        return Err(refused(
            "it unpacked to a different size than was announced",
        ));
    }

    let record = Record::from_json(&json).map_err(|e| refused(&e.to_string()))?;
    drop(json);
    // A scan is always the first revision; revisions are made on this laptop.
    if record.revision != 1 || record.previous.is_some() {
        return Err(refused("it is not a fresh scan"));
    }
    if !undecided(&record) {
        return Err(refused(
            "it already says where people or shared things go, which only this laptop decides",
        ));
    }
    Ok(record)
}

/// Unpacks `packed`, stopping one byte past `raw_len`, so a small piece cannot unpack into a
/// flood (a compression bomb). Room for exactly that is made first, so the buffer never doubles
/// past it while growing.
fn unpack(packed: &[u8], raw_len: u64) -> Result<Vec<u8>, TransferError> {
    let room =
        usize::try_from(raw_len.saturating_add(1)).map_err(|_| refused("it is too large"))?;
    let mut json = Vec::with_capacity(room);
    zstd::stream::read::Decoder::with_buffer(packed)
        .map_err(|_| refused("it could not be unpacked"))?
        .take(raw_len + 1)
        .read_to_end(&mut json)
        .map_err(|_| refused("it could not be unpacked"))?;
    Ok(json)
}

/// Whether `record` leaves every choice about people and shared things to the new laptop.
fn undecided(record: &Record) -> bool {
    record.mapping == Record::new(record.source_laptop).mapping
}

fn refused(why: &str) -> TransferError {
    TransferError::Inventory(why.to_string())
}

async fn send(ch: &mut impl Channel, data: &[u8]) -> Result<(), TransferError> {
    ch.send(data)
        .await
        .map_err(|_| TransferError::ConnectionDropped)
}

/// The next frame, if it arrives within [`INVENTORY_PIECE_WAIT`].
async fn recv_in_time(ch: &mut impl Channel) -> Result<Vec<u8>, TransferError> {
    tokio::time::timeout(INVENTORY_PIECE_WAIT, ch.recv())
        .await
        .map_err(|_| TransferError::NoAnswer("its list stopped arriving".into()))?
        .map_err(|_| TransferError::ConnectionDropped)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChannelError;

    /// A connection that must never be used.
    struct Unused;

    impl Channel for Unused {
        async fn send(&mut self, _: &[u8]) -> Result<(), ChannelError> {
            panic!("nothing should be sent")
        }
        async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
            panic!("nothing should be received")
        }
    }

    #[tokio::test]
    async fn a_record_over_the_limit_is_refused_before_anything_is_sent() {
        let record = Record::new(
            pctwin_record::LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap(),
        );
        let len = record.to_json().len() as u64;
        let got = send_within(&mut Unused, &record, len - 1).await;
        assert!(matches!(got, Err(TransferError::Inventory(_))), "{got:?}");
    }

    #[test]
    fn unpacking_stops_one_byte_past_the_announced_size() {
        let bomb = zstd::bulk::compress(&vec![b'x'; 1_000_000], 19).unwrap();
        assert_eq!(unpack(&bomb, 100).unwrap().len(), 101);
        assert_eq!(unpack(&bomb, 2_000_000).unwrap().len(), 1_000_000);
    }
}
