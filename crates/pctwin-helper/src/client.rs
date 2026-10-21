//! The app's side of the conversation with the helper: open the session, scan an approved person,
//! read their files by handle, say goodbye. Every answer is checked to be the one each request
//! has; anything else ends the conversation.

use std::io::{Read, Write};

use pctwin_record::Item;

use crate::protocol::{
    Answer, FrameError, FromHelper, Handle, Refusal, Request, read_from_helper, send_request,
};

/// Why the helper could not do what the app asked.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error(transparent)]
    Frame(#[from] FrameError),
    #[error("the helper refused: {0}")]
    Refused(Refusal),
    #[error("the helper answered out of turn")]
    OutOfTurn,
}

/// A person the helper scanned: their totals and their files, each with its handle. The items
/// travel in the move record and are never shown to the mover (decided 21 October 2026).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scanned {
    pub files: u64,
    pub bytes: u64,
    pub items: Vec<(Item, Handle)>,
}

/// An open session with the helper.
pub struct Client<S: Read + Write> {
    stream: S,
}

impl<S: Read + Write> Client<S> {
    /// Opens the session the helper was started for, with its token.
    pub fn open(mut stream: S, token: &str) -> Result<Self, ClientError> {
        send_request(
            &mut stream,
            &Request::Hello {
                session: token.to_string(),
            },
        )?;
        let mut c = Self { stream };
        c.expect_answer(|a| matches!(a, Answer::Ready))?;
        Ok(c)
    }

    fn next(&mut self) -> Result<FromHelper, ClientError> {
        Ok(read_from_helper(&mut self.stream)?)
    }

    fn expect_answer(&mut self, ok: impl Fn(&Answer) -> bool) -> Result<Answer, ClientError> {
        match self.next()? {
            FromHelper::Answer(Answer::Refused(why)) => Err(ClientError::Refused(why)),
            FromHelper::Answer(a) if ok(&a) => Ok(a),
            _ => Err(ClientError::OutOfTurn),
        }
    }

    /// Scans the approved person `account`.
    pub fn scan_person(&mut self, account: &str) -> Result<Scanned, ClientError> {
        send_request(
            &mut self.stream,
            &Request::ScanPerson {
                account: account.to_string(),
            },
        )?;
        let Answer::Scanned { files, bytes } =
            self.expect_answer(|a| matches!(a, Answer::Scanned { .. }))?
        else {
            return Err(ClientError::OutOfTurn);
        };
        let FromHelper::Handles(handles) = self.next()? else {
            return Err(ClientError::OutOfTurn);
        };
        if handles.len() as u64 != files {
            return Err(ClientError::OutOfTurn);
        }
        let mut items: Vec<Item> = Vec::with_capacity(handles.len());
        while items.len() < handles.len() {
            let FromHelper::Items(page) = self.next()? else {
                return Err(ClientError::OutOfTurn);
            };
            if page.is_empty() || items.len() + page.len() > handles.len() {
                return Err(ClientError::OutOfTurn);
            }
            items.extend(page);
        }
        Ok(Scanned {
            files,
            bytes,
            items: items.into_iter().zip(handles).collect(),
        })
    }

    /// Reads `len` bytes from `offset` of the file behind `handle`.
    pub fn read(&mut self, handle: u64, offset: u64, len: u32) -> Result<Vec<u8>, ClientError> {
        send_request(
            &mut self.stream,
            &Request::ReadBlocks {
                handle,
                offset,
                len,
            },
        )?;
        match self.next()? {
            FromHelper::Blocks(b) if b.len() == len as usize => Ok(b),
            FromHelper::Answer(Answer::Refused(why)) => Err(ClientError::Refused(why)),
            _ => Err(ClientError::OutOfTurn),
        }
    }

    /// Ends the session: the helper ends with it.
    pub fn bye(mut self) -> Result<(), ClientError> {
        send_request(&mut self.stream, &Request::Bye)?;
        self.expect_answer(|a| matches!(a, Answer::Goodbye))?;
        Ok(())
    }
}
