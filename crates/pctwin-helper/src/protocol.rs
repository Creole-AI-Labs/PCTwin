//! What the app and the helper say to each other over the private channel: a fixed list of typed
//! commands, never a shell or a free-form command (Security Design part C). Every message is a
//! frame: its length (4 bytes, big-endian) then its body, never longer than [`MAX_FRAME`].
//!
//! - The app's requests are JSON, at most [`MAX_REQUEST`] bytes, with no field the list does
//!   not name.
//! - The helper's answers start with one byte saying what follows: a JSON answer, the blocks
//!   read, or the handles of a person's files (16 bytes each: the handle, then the size).

use std::io::{self, Read, Write};

use serde::{Deserialize, Serialize};

/// Longest request, in bytes: commands are short.
pub const MAX_REQUEST: usize = 64 * 1024;
/// Longest read in one `ReadBlocks`: one block of the largest size a move uses.
pub const MAX_READ: u32 = 16 * 1024 * 1024;
/// Most files one person's scan hands out (the move record's own limit is lower).
pub const MAX_HANDLES: usize = 1_000_000;
/// Longest frame either way: the handles of [`MAX_HANDLES`] files, with room for its header.
pub const MAX_FRAME: usize = 1 + 4 + MAX_HANDLES * 16;
/// Items in one frame of a person's items (well inside [`MAX_FRAME`] however long their names).
pub const ITEMS_PER_FRAME: usize = 1000;

/// What the app asks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum Request {
    /// First, and only first: the session this helper was started for, by its token.
    Hello { session: String },
    /// Find the approved person `account`'s files (read-only) and keep them in the helper; the
    /// app gets their totals and a handle for each file, never their names.
    ScanPerson { account: String },
    /// Read `len` bytes from `offset` of the file behind `handle`.
    ReadBlocks { handle: u64, offset: u64, len: u32 },
    /// New laptop: make the approved person's standard account `name`, shown as `full_name`,
    /// with a temporary password they must change at their first sign-in.
    CreateAccount { name: String, full_name: String },
    /// New laptop: the folder in this move's holding area where the approved person `account`'s
    /// files are put during the move (made the first time it is asked for).
    HoldFor { account: String },
    /// New laptop: hand the approved person `account`'s held files over into their own folders,
    /// now (the administrator's way, at the end of the move).
    HandOverNow { account: String },
    /// New laptop: hand the approved person `account`'s held files over when they next sign in,
    /// as them (the usual way once the move is over).
    HandOverAtSignIn { account: String },
    /// The move is over: the helper ends.
    Bye,
}

/// Why the helper did not do what it was asked: only what the app needs to say what happened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum Refusal {
    #[error("the helper was not started for this session")]
    WrongSession,
    #[error("the session has not been opened yet")]
    NoSession,
    #[error("that person was not approved for this move")]
    NotApproved,
    #[error("no such file in the helper's plan")]
    UnknownHandle,
    #[error("asked to read more than one block at once")]
    TooMuch,
    #[error("asked to read past the end of the file")]
    PastTheEnd,
    #[error("it is stored online only, so it was not downloaded to be moved")]
    OnlineOnly,
    #[error("another program has it open, so it was not read; try again once it is closed")]
    InUse,
    #[error("it changed since the scan, so it was not read; scan it again")]
    Changed,
    #[error("it is no longer there")]
    Gone,
    #[error("it is not a plain file (a link or a special entry), so it was not read")]
    NotPlain,
    #[error("it could not be read")]
    Unreadable,
    #[error("that person's home folder could not be found")]
    NoHome,
    #[error("too many files to hand out at once")]
    TooManyFiles,
    #[error("an account with that name already exists, and was left as it is")]
    AccountExists,
    #[error("the account could not be made")]
    AccountNotMade,
    #[error("nothing is being held for that person")]
    NothingHeld,
}

/// The helper's JSON answers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum Answer {
    Ready,
    /// The account was made: its ID, and the temporary password, said once and never kept by
    /// the helper. The app shows it to the approving administrator in a box of the system's own
    /// (never on PCTwin's screens) and keeps it nowhere.
    AccountMade {
        account: String,
        password: String,
        /// macOS: whether the account can unlock FileVault (`false`: turn it on in System
        /// Settings, Users & Groups). Always `true` elsewhere.
        secure_token: bool,
    },
    /// Where the person's files are put during the move.
    Holding {
        path: String,
    },
    /// The person's files handed over: how many landed, were kept aside (a name taken) or failed.
    HandedOver {
        landed: u64,
        kept: u64,
        failed: u64,
    },
    /// The person's files will be handed over at their next sign-in.
    AtSignIn,
    /// The person's totals; the handles follow in their own frame.
    Scanned {
        files: u64,
        bytes: u64,
    },
    Refused(Refusal),
    Goodbye,
}

/// One file of a person's scan as the app sees it: a handle, and its size at the scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Handle {
    pub handle: u64,
    pub size: u64,
}

const ANSWER: u8 = 1;
const BLOCKS: u8 = 2;
const HANDLES: u8 = 3;
const ITEMS: u8 = 4;

/// A frame read off the channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FromHelper {
    Answer(Answer),
    Blocks(Vec<u8>),
    Handles(Vec<Handle>),
    /// A page of a person's items, in their handles' order.
    Items(Vec<pctwin_record::Item>),
}

/// Why a frame could not be read or written.
#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    #[error("the channel failed: {0}")]
    Io(#[from] io::Error),
    #[error("a frame is too long")]
    TooLong,
    #[error("a frame could not be read: {0}")]
    Bad(String),
}

pub fn write_frame(w: &mut impl Write, body: &[u8]) -> Result<(), FrameError> {
    if body.len() > MAX_FRAME {
        return Err(FrameError::TooLong);
    }
    let len = u32::try_from(body.len()).map_err(|_| FrameError::TooLong)?;
    w.write_all(&len.to_be_bytes())?;
    w.write_all(body)?;
    w.flush()?;
    Ok(())
}

/// Reads one frame, refusing one longer than `limit` before reading its body.
pub fn read_frame(r: &mut impl Read, limit: usize) -> Result<Vec<u8>, FrameError> {
    let mut len = [0u8; 4];
    r.read_exact(&mut len)?;
    let len = u32::from_be_bytes(len) as usize;
    if len > limit {
        return Err(FrameError::TooLong);
    }
    let mut body = vec![0u8; len];
    r.read_exact(&mut body)?;
    Ok(body)
}

pub fn send_request(w: &mut impl Write, request: &Request) -> Result<(), FrameError> {
    let body = serde_json::to_vec(request).map_err(|e| FrameError::Bad(e.to_string()))?;
    if body.len() > MAX_REQUEST {
        return Err(FrameError::TooLong);
    }
    write_frame(w, &body)
}

pub fn read_request(r: &mut impl Read) -> Result<Request, FrameError> {
    let body = read_frame(r, MAX_REQUEST)?;
    serde_json::from_slice(&body).map_err(|e| FrameError::Bad(e.to_string()))
}

pub fn send_answer(w: &mut impl Write, answer: &Answer) -> Result<(), FrameError> {
    let mut body = vec![ANSWER];
    serde_json::to_writer(&mut body, answer).map_err(|e| FrameError::Bad(e.to_string()))?;
    write_frame(w, &body)
}

pub fn send_blocks(w: &mut impl Write, bytes: &[u8]) -> Result<(), FrameError> {
    let mut body = Vec::with_capacity(1 + bytes.len());
    body.push(BLOCKS);
    body.extend_from_slice(bytes);
    write_frame(w, &body)
}

pub fn send_handles(w: &mut impl Write, handles: &[Handle]) -> Result<(), FrameError> {
    if handles.len() > MAX_HANDLES {
        return Err(FrameError::TooLong);
    }
    let mut body = Vec::with_capacity(1 + handles.len() * 16);
    body.push(HANDLES);
    for h in handles {
        body.extend_from_slice(&h.handle.to_be_bytes());
        body.extend_from_slice(&h.size.to_be_bytes());
    }
    write_frame(w, &body)
}

pub fn send_items(w: &mut impl Write, items: &[&pctwin_record::Item]) -> Result<(), FrameError> {
    let mut body = vec![ITEMS];
    serde_json::to_writer(&mut body, items).map_err(|e| FrameError::Bad(e.to_string()))?;
    write_frame(w, &body)
}

pub fn read_from_helper(r: &mut impl Read) -> Result<FromHelper, FrameError> {
    let body = read_frame(r, MAX_FRAME)?;
    let (kind, rest) = body
        .split_first()
        .ok_or_else(|| FrameError::Bad("an empty frame".into()))?;
    match *kind {
        ANSWER => serde_json::from_slice(rest)
            .map(FromHelper::Answer)
            .map_err(|e| FrameError::Bad(e.to_string())),
        BLOCKS => Ok(FromHelper::Blocks(rest.to_vec())),
        HANDLES => {
            if rest.len() % 16 != 0 {
                return Err(FrameError::Bad("handles of a broken length".into()));
            }
            Ok(FromHelper::Handles(
                rest.as_chunks::<16>()
                    .0
                    .iter()
                    .map(|c| {
                        let (h, s) = c.split_at(8);
                        Handle {
                            handle: u64::from_be_bytes(h.try_into().unwrap_or_default()),
                            size: u64::from_be_bytes(s.try_into().unwrap_or_default()),
                        }
                    })
                    .collect(),
            ))
        }
        ITEMS => serde_json::from_slice(rest)
            .map(FromHelper::Items)
            .map_err(|e| FrameError::Bad(e.to_string())),
        _ => Err(FrameError::Bad("an unknown kind of frame".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_and_answers_cross_whole() {
        let mut wire = Vec::new();
        let asked = [
            Request::Hello {
                session: "t".into(),
            },
            Request::ScanPerson {
                account: "S-1-5-21-1".into(),
            },
            Request::ReadBlocks {
                handle: 7,
                offset: 1 << 40,
                len: MAX_READ,
            },
            Request::Bye,
        ];
        for r in &asked {
            send_request(&mut wire, r).unwrap();
        }
        let mut read = wire.as_slice();
        for r in &asked {
            assert_eq!(&read_request(&mut read).unwrap(), r);
        }
        let mut wire = Vec::new();
        send_answer(&mut wire, &Answer::Refused(Refusal::InUse)).unwrap();
        send_blocks(&mut wire, b"abc").unwrap();
        let handles = [Handle { handle: 1, size: 2 }, Handle { handle: 3, size: 4 }];
        send_handles(&mut wire, &handles).unwrap();
        let mut read = wire.as_slice();
        assert_eq!(
            read_from_helper(&mut read).unwrap(),
            FromHelper::Answer(Answer::Refused(Refusal::InUse))
        );
        assert_eq!(
            read_from_helper(&mut read).unwrap(),
            FromHelper::Blocks(b"abc".to_vec())
        );
        assert_eq!(
            read_from_helper(&mut read).unwrap(),
            FromHelper::Handles(handles.to_vec())
        );
    }

    #[test]
    fn anything_but_the_fixed_commands_is_refused() {
        for bad in [
            &br#"{"run":{"command":"cmd /c whoami"}}"#[..],
            br#"{"read-blocks":{"handle":1,"offset":0,"len":1,"path":"C:/x"}}"#,
            br#"{"scan-person":{}}"#,
            br#"not json"#,
        ] {
            let mut wire = Vec::new();
            write_frame(&mut wire, bad).unwrap();
            assert!(read_request(&mut wire.as_slice()).is_err(), "{bad:?}");
        }
        // A frame longer than a request may be is refused before its body is read.
        let mut wire = ((MAX_REQUEST + 1) as u32).to_be_bytes().to_vec();
        wire.extend(std::iter::repeat_n(b' ', 8));
        assert!(matches!(
            read_request(&mut wire.as_slice()),
            Err(FrameError::TooLong)
        ));
    }
}
