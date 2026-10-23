//! The app's side of the conversation with the helper: open the session, scan an approved person,
//! read their files by handle; on the new laptop make accounts, hold each person's files and hand
//! them over; say goodbye. Every answer is checked to be the one each request has; anything else
//! ends the conversation.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

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
    #[error("could not reach the helper: {0}")]
    Unreachable(String),
}

/// The private channel to the helper: Windows, its named pipe; Mac and Linux, its socket.
#[cfg(windows)]
pub type Channel = std::fs::File;
/// The private channel to the helper: Windows, its named pipe; Mac and Linux, its socket.
#[cfg(unix)]
pub type Channel = std::os::unix::net::UnixStream;

/// Connects to the helper started for this move and opens its session with `token`. Windows: the
/// named pipe `channel` (`dir` unused). Mac and Linux: the socket `channel` in the folder `dir`,
/// refused unless the system's own account (root) serves it, so a socket planted by anyone else
/// is never spoken to.
pub fn connect(
    channel: &str,
    dir: Option<&Path>,
    token: &str,
) -> Result<Client<Channel>, ClientError> {
    let unreachable = |e: pctwin_helper_os::Error| ClientError::Unreachable(e.to_string());
    #[cfg(windows)]
    let stream = {
        let _ = dir;
        let name = pctwin_helper_os::PipeName::parse(channel).map_err(unreachable)?;
        let pipe = pctwin_helper_os::PipeClient::connect(&name).map_err(unreachable)?;
        pipe.stream()
            .try_clone()
            .map_err(|e| ClientError::Unreachable(e.to_string()))?
    };
    #[cfg(unix)]
    let stream = {
        let dir = dir.ok_or_else(|| ClientError::Unreachable("no channel folder".into()))?;
        pctwin_helper_os::connect(&dir.join(channel), 0).map_err(unreachable)?
    };
    // The helper's first answer comes at once, or never: a channel that does not answer is not
    // waited on for ever (Mac and Linux; a Windows pipe has no time limit to set).
    #[cfg(unix)]
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(30)))
        .map_err(|e| ClientError::Unreachable(e.to_string()))?;
    let client = Client::open(stream, token)?;
    #[cfg(unix)]
    client
        .stream
        .set_read_timeout(None)
        .map_err(|e| ClientError::Unreachable(e.to_string()))?;
    Ok(client)
}

/// macOS: connects to the launch daemon's socket (refused unless root serves it), opens the move
/// with the administrator's `approval` for the people `approve` on the laptop `laptop`, then the
/// session with `token`. A refusal (the approval not confirmed) comes back as
/// [`ClientError::Refused`].
#[cfg(unix)]
pub fn connect_daemon(
    approval: &pctwin_helper_os::approval::Approval,
    token: &str,
    approve: &[String],
    laptop: &pctwin_record::LaptopId,
) -> Result<Client<Channel>, ClientError> {
    use pctwin_helper_os::daemon::{CHANNEL_DIR, CHANNEL_SOCKET};
    let path = Path::new(CHANNEL_DIR).join(CHANNEL_SOCKET);
    let mut stream =
        pctwin_helper_os::connect(&path, 0).map_err(|e| ClientError::Unreachable(e.to_string()))?;
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(30)))
        .map_err(|e| ClientError::Unreachable(e.to_string()))?;
    send_request(
        &mut stream,
        &Request::Open {
            approval: crate::opening::approval_to_hex(approval),
            session: token.to_string(),
            approve: approve.to_vec(),
            laptop: laptop.to_hex(),
        },
    )?;
    let client = Client::open(stream, token)?;
    client
        .stream
        .set_read_timeout(None)
        .map_err(|e| ClientError::Unreachable(e.to_string()))?;
    Ok(client)
}

/// An account the helper made on the new laptop.
pub struct MadeAccount {
    /// The system's ID for it.
    pub account: String,
    /// The temporary password, changed by the person at their first sign-in; shown once, in the
    /// system's own window, never stored.
    pub password: zeroize::Zeroizing<String>,
    /// macOS: whether it can unlock FileVault. Always `true` elsewhere.
    pub secure_token: bool,
}

/// What a hand-over came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandedOver {
    pub landed: u64,
    /// Kept aside: a file of that name was already there.
    pub kept: u64,
    pub failed: u64,
}

/// A person the helper scanned: their totals and their files, each with its handle. The items
/// travel in the move record and are never shown to the mover (decided 21 October 2026).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scanned {
    pub files: u64,
    pub bytes: u64,
    pub items: Vec<(Item, Handle)>,
    /// macOS: places of theirs a privacy setting kept the helper out of (the helper needs Full
    /// Disk Access).
    pub privacy_blocked: u64,
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
        let Answer::Scanned {
            files,
            bytes,
            privacy_blocked,
        } = self.expect_answer(|a| matches!(a, Answer::Scanned { .. }))?
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
            privacy_blocked,
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

    /// New laptop: makes a standard account `name` (shown as `full_name`) with a temporary
    /// password.
    pub fn create_account(
        &mut self,
        name: &str,
        full_name: &str,
    ) -> Result<MadeAccount, ClientError> {
        send_request(
            &mut self.stream,
            &Request::CreateAccount {
                name: name.to_string(),
                full_name: full_name.to_string(),
            },
        )?;
        match self.expect_answer(|a| matches!(a, Answer::AccountMade { .. }))? {
            Answer::AccountMade {
                account,
                password,
                secure_token,
            } => Ok(MadeAccount {
                account,
                password: zeroize::Zeroizing::new(password),
                secure_token,
            }),
            _ => Err(ClientError::OutOfTurn),
        }
    }

    /// New laptop: the folder where the approved person `account`'s files are put during the
    /// move.
    pub fn hold_for(&mut self, account: &str) -> Result<PathBuf, ClientError> {
        send_request(
            &mut self.stream,
            &Request::HoldFor {
                account: account.to_string(),
            },
        )?;
        match self.expect_answer(|a| matches!(a, Answer::Holding { .. }))? {
            Answer::Holding { path } => Ok(PathBuf::from(path)),
            _ => Err(ClientError::OutOfTurn),
        }
    }

    /// New laptop: hands the approved person `account`'s held files over into their own folders,
    /// now.
    pub fn hand_over_now(&mut self, account: &str) -> Result<HandedOver, ClientError> {
        send_request(
            &mut self.stream,
            &Request::HandOverNow {
                account: account.to_string(),
            },
        )?;
        match self.expect_answer(|a| matches!(a, Answer::HandedOver { .. }))? {
            Answer::HandedOver {
                landed,
                kept,
                failed,
            } => Ok(HandedOver {
                landed,
                kept,
                failed,
            }),
            _ => Err(ClientError::OutOfTurn),
        }
    }

    /// New laptop: hands the approved person `account`'s held files over at their next sign-in.
    pub fn hand_over_at_sign_in(&mut self, account: &str) -> Result<(), ClientError> {
        send_request(
            &mut self.stream,
            &Request::HandOverAtSignIn {
                account: account.to_string(),
            },
        )?;
        self.expect_answer(|a| matches!(a, Answer::AtSignIn))?;
        Ok(())
    }

    /// Ends the session: the helper ends with it.
    pub fn bye(mut self) -> Result<(), ClientError> {
        send_request(&mut self.stream, &Request::Bye)?;
        self.expect_answer(|a| matches!(a, Answer::Goodbye))?;
        Ok(())
    }
}
