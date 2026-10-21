//! One session of the helper: one app, one move, one connection. The app must say the session's
//! token first; the helper then scans only the people the administrator approved, keeps their
//! files in its own plan, and reads only from that plan, by handle. Nothing else is accepted, and
//! nothing a request says is ever taken as a path.

use std::collections::BTreeSet;
use std::io::{Read, Write};

use pctwin_gate::FileId;

use crate::protocol::{
    Answer, FrameError, Handle, MAX_HANDLES, Refusal, Request, read_request, send_answer,
    send_blocks, send_handles,
};
use crate::read::{Entry, read_range};

/// Finds an approved person's files (see [`crate::person::scan_person`]).
pub type Scanner = dyn FnMut(&str) -> Result<Vec<Entry>, Refusal>;

/// What the helper was started for.
pub struct Session<'s> {
    /// The token the app was given when it started the helper.
    token: String,
    /// The people the administrator approved, by account ID (never the mover).
    approved: BTreeSet<String>,
    opened: bool,
    /// Every file handed out, by handle (its index), with its identity once first read.
    plan: Vec<(Entry, Option<FileId>)>,
    scanner: &'s mut Scanner,
}

impl<'s> Session<'s> {
    pub fn new(token: String, approved: BTreeSet<String>, scanner: &'s mut Scanner) -> Self {
        Self {
            token,
            approved,
            opened: false,
            plan: Vec::new(),
            scanner,
        }
    }

    /// Serves the app on `channel` until it says goodbye or the channel closes.
    pub fn serve(&mut self, channel: &mut (impl Read + Write)) -> Result<(), FrameError> {
        loop {
            let request = match read_request(channel) {
                Ok(r) => r,
                // The app went away: the move is over.
                Err(FrameError::Io(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    return Ok(());
                }
                Err(e) => return Err(e),
            };
            match request {
                Request::Hello { session } => {
                    // Compared whole; the token is random and only ever said to this helper.
                    if session != self.token || self.opened {
                        send_answer(channel, &Answer::Refused(Refusal::WrongSession))?;
                        return Ok(());
                    }
                    self.opened = true;
                    send_answer(channel, &Answer::Ready)?;
                }
                _ if !self.opened => {
                    send_answer(channel, &Answer::Refused(Refusal::NoSession))?;
                    return Ok(());
                }
                Request::ScanPerson { account } => self.scan(channel, &account)?,
                Request::ReadBlocks {
                    handle,
                    offset,
                    len,
                } => {
                    let read = usize::try_from(handle)
                        .ok()
                        .and_then(|i| self.plan.get_mut(i))
                        .ok_or(Refusal::UnknownHandle)
                        .and_then(|(entry, pinned)| read_range(entry, pinned, offset, len));
                    match read {
                        Ok(bytes) => send_blocks(channel, &bytes)?,
                        Err(why) => send_answer(channel, &Answer::Refused(why))?,
                    }
                }
                Request::Bye => {
                    send_answer(channel, &Answer::Goodbye)?;
                    return Ok(());
                }
            }
        }
    }

    fn scan(&mut self, channel: &mut impl Write, account: &str) -> Result<(), FrameError> {
        if !self.approved.contains(account) {
            return send_answer(channel, &Answer::Refused(Refusal::NotApproved));
        }
        let found = match (self.scanner)(account) {
            Ok(found) => found,
            Err(why) => return send_answer(channel, &Answer::Refused(why)),
        };
        if self.plan.len() + found.len() > MAX_HANDLES {
            return send_answer(channel, &Answer::Refused(Refusal::TooManyFiles));
        }
        let first = self.plan.len() as u64;
        let bytes = found.iter().fold(0u64, |t, e| t.saturating_add(e.size));
        let handles: Vec<Handle> = found
            .iter()
            .enumerate()
            .map(|(n, e)| Handle {
                handle: first + n as u64,
                size: e.size,
            })
            .collect();
        send_answer(
            channel,
            &Answer::Scanned {
                files: handles.len() as u64,
                bytes,
            },
        )?;
        send_handles(channel, &handles)?;
        self.plan.extend(found.into_iter().map(|e| (e, None)));
        Ok(())
    }
}
