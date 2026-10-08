use pctwin_record::ItemId;

use crate::{BlockMap, Header, MAX_BLOCK, ResumeTicket, Stamp, TransferError};

/// The largest piece of a block in one link message, leaving room for the message's own fields
/// within the link's 64 KiB limit.
pub const PIECE_MAX: usize = 60 * 1024;
/// The longest path inside a destination, in bytes (as the safety gate allows).
const MAX_PATH: usize = 4096;
/// The longest destination label (as the safety gate allows).
const MAX_LABEL: usize = 64;
/// A block's own header on the wire, before its contents.
pub(crate) const BLOCK_WIRE_OVERHEAD: usize = 46;
/// A whole block on the wire: the block's own header plus its largest contents.
const MAX_WIRE_BLOCK: usize = BLOCK_WIRE_OVERHEAD + MAX_BLOCK as usize;

/// One message of a transfer. `stream` tells files in flight apart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    /// Sender: a file starts, or continues after a drop (`resumed_done` is how many blocks the
    /// new laptop's resume ticket said it had; 0 for a fresh start).
    StartFile {
        stream: u32,
        item: ItemId,
        /// The approved destination's label on the new laptop.
        destination: String,
        /// The path inside that destination.
        path: String,
        header: Header,
        resumed_done: u64,
    },
    /// Sender: part of a block; `last` ends the block.
    Piece {
        stream: u32,
        last: bool,
        bytes: Vec<u8>,
    },
    /// Sender: the file's last block was sent.
    EndFile {
        stream: u32,
        stamp_after: Stamp,
        changed: bool,
    },
    /// Receiver: this block is written.
    Receipt { stream: u32, block: u64 },
    /// Receiver, after a new connection: continue this file from here.
    ResumeFrom { stream: u32, ticket: ResumeTicket },
    /// Receiver: the file is finished under its real name (`ok`), or was not.
    FileDone { stream: u32, ok: bool },
    /// Receiver, at the start of each connection: everything it already has has been listed
    /// (`ResumeFrom` and `FileDone`), so sending can begin.
    Ready,
    /// Sender: every file has been sent and answered for.
    AllSent,
    /// Receiver, answering every `StartFile`: no file of that name and size is here (`None`),
    /// or one is, with its BLAKE3 fingerprint.
    Have {
        stream: u32,
        same_size: Option<[u8; 32]>,
    },
    /// Sender: the file already there is identical, so it is not sent.
    Skip { stream: u32 },
    /// Receiver, in place of a receipt: that block was not written (the file failed, is not open,
    /// or belongs to an earlier attempt), so its file will not finish.
    Refused { stream: u32 },
}

const START: u8 = 1;
const PIECE: u8 = 2;
const END: u8 = 3;
const RECEIPT: u8 = 4;
const RESUME: u8 = 5;
const DONE: u8 = 6;
const READY: u8 = 7;
const ALL_SENT: u8 = 8;
const HAVE: u8 = 9;
const SKIP: u8 = 10;
const REFUSED: u8 = 11;

impl Message {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Vec::new();
        match self {
            Message::StartFile {
                stream,
                item,
                destination,
                path,
                header,
                resumed_done,
            } => {
                w.push(START);
                w.extend_from_slice(&stream.to_be_bytes());
                w.extend_from_slice(&item_bytes(item));
                put_header(&mut w, header);
                w.extend_from_slice(&resumed_done.to_be_bytes());
                put_text(&mut w, destination);
                put_text(&mut w, path);
            }
            Message::Piece {
                stream,
                last,
                bytes,
            } => {
                w.push(PIECE);
                w.extend_from_slice(&stream.to_be_bytes());
                w.push(u8::from(*last));
                w.extend_from_slice(bytes);
            }
            Message::EndFile {
                stream,
                stamp_after,
                changed,
            } => {
                w.push(END);
                w.extend_from_slice(&stream.to_be_bytes());
                put_stamp(&mut w, stamp_after);
                w.push(u8::from(*changed));
            }
            Message::Receipt { stream, block } => {
                w.push(RECEIPT);
                w.extend_from_slice(&stream.to_be_bytes());
                w.extend_from_slice(&block.to_be_bytes());
            }
            Message::ResumeFrom { stream, ticket } => {
                w.push(RESUME);
                w.extend_from_slice(&stream.to_be_bytes());
                w.extend_from_slice(&ticket.block_size.to_be_bytes());
                put_stamp(&mut w, &ticket.stamp);
                let map = ticket.done.encode();
                // A map's wire form is at most a few tens of KiB, which fits in a u32.
                w.extend_from_slice(&u32::try_from(map.len()).unwrap_or(u32::MAX).to_be_bytes());
                w.extend_from_slice(&map);
            }
            Message::FileDone { stream, ok } => {
                w.push(DONE);
                w.extend_from_slice(&stream.to_be_bytes());
                w.push(u8::from(*ok));
            }
            Message::Ready => {
                w.push(READY);
                w.extend_from_slice(&0u32.to_be_bytes());
            }
            Message::AllSent => {
                w.push(ALL_SENT);
                w.extend_from_slice(&0u32.to_be_bytes());
            }
            Message::Have { stream, same_size } => {
                w.push(HAVE);
                w.extend_from_slice(&stream.to_be_bytes());
                match same_size {
                    Some(hash) => {
                        w.push(1);
                        w.extend_from_slice(hash);
                    }
                    None => w.push(0),
                }
            }
            Message::Skip { stream } => {
                w.push(SKIP);
                w.extend_from_slice(&stream.to_be_bytes());
            }
            Message::Refused { stream } => {
                w.push(REFUSED);
                w.extend_from_slice(&stream.to_be_bytes());
            }
        }
        w
    }

    /// Reads a message, refusing anything unknown, cut short, with bytes left over, or outside
    /// its limits.
    pub fn decode(bytes: &[u8]) -> Result<Self, TransferError> {
        let mut r = Reader { bytes, at: 0 };
        let kind = r.u8()?;
        let stream = r.u32()?;
        let message = match kind {
            START => {
                let item = r.item()?;
                let header = r.header()?;
                let resumed_done = r.u64()?;
                let destination = r.text(MAX_LABEL)?;
                let path = r.text(MAX_PATH)?;
                Message::StartFile {
                    stream,
                    item,
                    destination,
                    path,
                    header,
                    resumed_done,
                }
            }
            PIECE => {
                let last = r.flag()?;
                let rest = r.rest();
                if rest.len() > PIECE_MAX {
                    return Err(damaged("piece too large"));
                }
                Message::Piece {
                    stream,
                    last,
                    bytes: rest.to_vec(),
                }
            }
            END => Message::EndFile {
                stream,
                stamp_after: r.stamp()?,
                changed: r.flag()?,
            },
            RECEIPT => Message::Receipt {
                stream,
                block: r.u64()?,
            },
            RESUME => Message::ResumeFrom {
                stream,
                ticket: ResumeTicket {
                    block_size: r.u64()?,
                    stamp: r.stamp()?,
                    done: r.map()?,
                },
            },
            DONE => Message::FileDone {
                stream,
                ok: r.flag()?,
            },
            READY if stream == 0 => Message::Ready,
            ALL_SENT if stream == 0 => Message::AllSent,
            HAVE => Message::Have {
                stream,
                same_size: if r.flag()? {
                    let mut hash = [0u8; 32];
                    hash.copy_from_slice(r.take(32)?);
                    Some(hash)
                } else {
                    None
                },
            },
            SKIP => Message::Skip { stream },
            REFUSED => Message::Refused { stream },
            _ => return Err(damaged("unknown message")),
        };
        r.end()?;
        Ok(message)
    }
}

/// Splits a whole block's wire form into pieces for `stream`; the last one is marked.
pub fn split_into_pieces(stream: u32, block: &[u8]) -> Vec<Message> {
    let chunks: Vec<&[u8]> = if block.is_empty() {
        vec![&[]]
    } else {
        block.chunks(PIECE_MAX).collect()
    };
    let count = chunks.len();
    chunks
        .into_iter()
        .enumerate()
        .map(|(i, c)| Message::Piece {
            stream,
            last: i + 1 == count,
            bytes: c.to_vec(),
        })
        .collect()
}

/// Rejoins one stream's pieces into whole blocks, never holding more than the largest block.
#[derive(Debug, Default)]
pub struct PieceBuffer {
    bytes: Vec<u8>,
}

impl PieceBuffer {
    /// Adds a piece; returns the whole block when `last` ends it.
    pub fn add(&mut self, piece: &[u8], last: bool) -> Result<Option<Vec<u8>>, TransferError> {
        if self.bytes.len() + piece.len() > MAX_WIRE_BLOCK {
            self.bytes.clear();
            return Err(damaged("a block grew past the largest allowed"));
        }
        self.bytes.extend_from_slice(piece);
        Ok(last.then(|| std::mem::take(&mut self.bytes)))
    }
}

fn damaged(why: &str) -> TransferError {
    TransferError::Damaged(why.to_string())
}

fn item_bytes(item: &ItemId) -> [u8; 16] {
    let mut out = [0u8; 16];
    // An ItemId's hex form is always 32 hex digits.
    let _ = hex_into(&item.to_hex(), &mut out);
    out
}

fn hex_into(hex: &str, out: &mut [u8]) -> Option<()> {
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(hex.get(i * 2..i * 2 + 2)?, 16).ok()?;
    }
    Some(())
}

fn put_header(w: &mut Vec<u8>, h: &Header) {
    w.extend_from_slice(&h.size.to_be_bytes());
    w.extend_from_slice(&h.block_size.to_be_bytes());
    w.extend_from_slice(&h.block_count.to_be_bytes());
    put_stamp(w, &h.stamp);
}

fn put_stamp(w: &mut Vec<u8>, s: &Stamp) {
    w.extend_from_slice(&s.size.to_be_bytes());
    match s.modified_ns {
        Some(m) => {
            w.push(1);
            w.extend_from_slice(&m.to_be_bytes());
        }
        None => w.push(0),
    }
}

fn put_text(w: &mut Vec<u8>, text: &str) {
    // Limits are checked when reading; lengths over u16 are clipped here and then refused.
    let len = u16::try_from(text.len()).unwrap_or(u16::MAX);
    w.extend_from_slice(&len.to_be_bytes());
    w.extend_from_slice(&text.as_bytes()[..usize::from(len)]);
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], TransferError> {
        let end = self.at.checked_add(n).ok_or_else(|| damaged("too long"))?;
        let out = self
            .bytes
            .get(self.at..end)
            .ok_or_else(|| damaged("cut short"))?;
        self.at = end;
        Ok(out)
    }

    fn u8(&mut self) -> Result<u8, TransferError> {
        Ok(self.take(1)?[0])
    }

    fn flag(&mut self) -> Result<bool, TransferError> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(damaged("bad flag")),
        }
    }

    fn u32(&mut self) -> Result<u32, TransferError> {
        let b = self.take(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn u64(&mut self) -> Result<u64, TransferError> {
        let mut a = [0u8; 8];
        a.copy_from_slice(self.take(8)?);
        Ok(u64::from_be_bytes(a))
    }

    fn i64(&mut self) -> Result<i64, TransferError> {
        let mut a = [0u8; 8];
        a.copy_from_slice(self.take(8)?);
        Ok(i64::from_be_bytes(a))
    }

    fn item(&mut self) -> Result<ItemId, TransferError> {
        let b = self.take(16)?;
        let hex: String = b.iter().map(|x| format!("{x:02x}")).collect();
        ItemId::from_hex(&hex).map_err(|_| damaged("bad item"))
    }

    fn stamp(&mut self) -> Result<Stamp, TransferError> {
        let size = self.u64()?;
        let modified_ns = if self.flag()? {
            Some(self.i64()?)
        } else {
            None
        };
        Ok(Stamp { size, modified_ns })
    }

    fn header(&mut self) -> Result<Header, TransferError> {
        Ok(Header {
            size: self.u64()?,
            block_size: self.u64()?,
            block_count: self.u64()?,
            stamp: self.stamp()?,
        })
    }

    fn map(&mut self) -> Result<BlockMap, TransferError> {
        // The map's own reader refuses more runs than a ticket carries.
        let len = self.u32()? as usize;
        BlockMap::decode(self.take(len)?)
    }

    fn text(&mut self, max: usize) -> Result<String, TransferError> {
        let b = self.take(2)?;
        let len = usize::from(u16::from_be_bytes([b[0], b[1]]));
        if len > max {
            return Err(damaged("text too long"));
        }
        let raw = self.take(len)?;
        String::from_utf8(raw.to_vec()).map_err(|_| damaged("not valid text"))
    }

    fn rest(&mut self) -> &'a [u8] {
        let out = &self.bytes[self.at..];
        self.at = self.bytes.len();
        out
    }

    fn end(&self) -> Result<(), TransferError> {
        if self.at == self.bytes.len() {
            Ok(())
        } else {
            Err(damaged("bytes left over"))
        }
    }
}
